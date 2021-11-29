/*******************************************************************************
*   (c) 2021 Zondax GmbH
*
*  Licensed under the Apache License, Version 2.0 (the "License");
*  you may not use this file except in compliance with the License.
*  You may obtain a copy of the License at
*
*      http://www.apache.org/licenses/LICENSE-2.0
*
*  Unless required by applicable law or agreed to in writing, software
*  distributed under the License is distributed on an "AS IS" BASIS,
*  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
*  See the License for the specific language governing permissions and
*  limitations under the License.
********************************************************************************/
use core::convert::TryFrom;

use crate::sys::{ViewError, PIC};

#[repr(u8)]
pub enum ZPacketType {
    Init = 0,
    Add = 1,
    Last = 2,
}

impl std::convert::TryFrom<u8> for ZPacketType {
    type Error = ();

    fn try_from(from: u8) -> Result<Self, ()> {
        match from {
            0 => Ok(Self::Init),
            1 => Ok(Self::Add),
            2 => Ok(Self::Last),
            _ => Err(()),
        }
    }
}

impl From<ZPacketType> for u8 {
    fn from(from: ZPacketType) -> Self {
        from as _
    }
}

#[repr(u8)]
pub enum LegacyPacketType {
    Init = 0,
    Add = 1,
    HashOnlyNext = 3,
    InitAndLast = 0x80,
    AddAndLast = 0x81,
    HashAndLast = 0x83,
}

impl std::convert::TryFrom<u8> for LegacyPacketType {
    type Error = ();

    fn try_from(from: u8) -> Result<Self, ()> {
        match from {
            0 => Ok(Self::Init),
            1 => Ok(Self::Add),
            3 => Ok(Self::HashOnlyNext),
            0x80 => Ok(Self::InitAndLast),
            0x81 => Ok(Self::AddAndLast),
            0x83 => Ok(Self::HashAndLast),
            _ => Err(()),
        }
    }
}

impl From<LegacyPacketType> for u8 {
    fn from(from: LegacyPacketType) -> Self {
        from as _
    }
}

pub trait PacketType {
    fn is_init(&self) -> bool;
    fn is_last(&self) -> bool;

    fn is_next(&self) -> bool {
        !self.is_init() && !self.is_last()
    }
}

impl PacketType for ZPacketType {
    fn is_init(&self) -> bool {
        matches!(self, Self::Init)
    }

    fn is_last(&self) -> bool {
        matches!(self, Self::Last)
    }
}

impl PacketType for LegacyPacketType {
    fn is_init(&self) -> bool {
        matches!(self, Self::Init) || matches!(self, Self::InitAndLast)
    }

    fn is_last(&self) -> bool {
        matches!(
            self,
            Self::InitAndLast | Self::HashAndLast | Self::AddAndLast
        )
    }
}

/// Utility struct to encapsulate the different packet types
pub enum PacketTypes {
    Z(ZPacketType),
    Legacy(LegacyPacketType),
}

impl PacketTypes {
    pub fn new(p1: u8, is_legacy: bool) -> Result<Self, ()> {
        if !is_legacy {
            Self::new_z(p1)
        } else {
            Self::new_legacy(p1)
        }
    }

    pub fn new_z(p1: u8) -> Result<Self, ()> {
        ZPacketType::try_from(p1).map(Self::Z)
    }

    pub fn new_legacy(p1: u8) -> Result<Self, ()> {
        LegacyPacketType::try_from(p1).map(Self::Legacy)
    }

    pub fn try_either(p1: u8) -> Result<Self, ()> {
        Self::new(p1, false).or_else(|_| Self::new(p1, true))
    }
}

impl PacketType for PacketTypes {
    fn is_init(&self) -> bool {
        match self {
            Self::Z(z) => z.is_init(),
            Self::Legacy(l) => l.is_init(),
        }
    }

    fn is_last(&self) -> bool {
        match self {
            Self::Z(z) => z.is_last(),
            Self::Legacy(l) => l.is_last(),
        }
    }

    fn is_next(&self) -> bool {
        match self {
            Self::Z(z) => z.is_next(),
            Self::Legacy(l) => l.is_next(),
        }
    }
}

#[inline(never)]
pub fn sha256x2(pieces: &[&[u8]], out: &mut [u8; 4]) -> Result<(), bolos::Error> {
    use crate::sys::{
        self,
        hash::{Hasher, Sha256},
    };

    sys::zemu_log_stack("sha256x2\x00");

    let mut hasher = {
        let mut loc = core::mem::MaybeUninit::<Sha256>::uninit();
        Sha256::new_gce(&mut loc)?;

        //Safety: we just initialized it above
        unsafe { loc.assume_init() }
    };

    for p in pieces {
        hasher.update(p)?;
    }

    let mut digest = [0; 32];
    //write first sha256
    hasher.finalize_dirty_into(&mut digest)?;

    //reset and feed the computed hash
    hasher.reset()?;
    hasher.update(&digest[..])?;

    //finalize the hash of the hash
    hasher.finalize_into(&mut digest)?;

    //copy only the first 4 bytes
    out.copy_from_slice(&digest[..4]);

    Ok(())
}

#[inline(never)]
pub fn handle_ui_message(item: &[u8], out: &mut [u8], page: u8) -> Result<u8, ViewError> {
    crate::sys::zemu_log_stack("handle_ui_message\x00");
    let m_len = out.len() - 1; //null byte terminator
    if m_len <= item.len() {
        let chunk = item
            .chunks(m_len) //divide in non-overlapping chunks
            .nth(page as usize) //get the nth chunk
            .ok_or(ViewError::Unknown)?;

        out[..chunk.len()].copy_from_slice(chunk);
        out[chunk.len()] = 0; //null terminate

        let n_pages = item.len() / m_len;
        Ok(1 + n_pages as u8)
    } else {
        out[..item.len()].copy_from_slice(item);
        out[item.len()] = 0; //null terminate
        Ok(1)
    }
}

#[cfg_attr(any(test, feature = "derive-debug"), derive(Debug))]
pub enum IntStrToFpStrError {
    BufferFull,
    BufferTooShort,
    /// Digit at .0 is not an ascii number
    NotAllDigitsAreNumbers(usize),
}

/// Return the len of the string until null termination
fn strlen(bytes: &[u8]) -> usize {
    bytes.split(|&n| n == 0).next().unwrap_or(bytes).len()
}

#[inline(never)]
/// Converts an integer number string
/// to a fixed point number string, in place
///
/// Returns Ok(subslice) which is the subslice with actual content,
/// trimming excess bytes
pub fn intstr_to_fpstr_inplace(
    s: &mut [u8],
    decimals: usize,
) -> Result<&mut [u8], IntStrToFpStrError> {
    //find the length of the string
    // if no 0s are found then the entire string is full with digits
    // so we return error
    let mut num_chars = strlen(s);

    if num_chars == s.len() {
        return Err(IntStrToFpStrError::BufferFull);
    }

    if s.is_empty() {
        return Err(IntStrToFpStrError::BufferTooShort);
    }

    //empty input string
    // let's just write a 0
    if num_chars == 0 {
        s[0] = b'0';
        num_chars = 1;
    }

    let mut first_digit_idx = None;
    //check that all are ascii numbers
    // and first the first digit
    let number_ascii_range = PIC::new(&(b'0'..=b'9')).into_inner();
    for (i, c) in s[..num_chars].iter_mut().enumerate() {
        if !number_ascii_range.contains(c) {
            return Err(IntStrToFpStrError::NotAllDigitsAreNumbers(i));
        }

        //just find the first digit
        if *c != b'0' {
            first_digit_idx = Some(i);
            break;
        }
    }

    //if we have a first digit
    if let Some(idx) = first_digit_idx {
        //move first_digit.. to the front
        s.copy_within(idx.., 0);

        //zero out the remaining
        s[num_chars - idx..].fill(0);

        //same as strlen(s)
        //we know where the string ends
        num_chars = num_chars - idx;
    } else {
        //if the first digit wasn't found
        // then it's just all 0s
        //we trim all the 0s after the first one
        s[1..].fill(0);
    }

    //we can return early if we have no decimals
    if decimals == 0 {
        num_chars = strlen(s);
        return Ok(&mut s[..num_chars]);
    }

    // Now insert decimal point

    //        0123456789012     <-decimal places
    //        abcd              < numChars = 4
    //                 abcd     < shift
    //        000000000abcd     < fill
    //        0.00000000abcd    < add decimal point

    if num_chars < decimals + 1 {
        // Move to end
        let padding = decimals - num_chars + 1;
        s.copy_within(..num_chars, padding);

        //fill the front with zeros
        s[..padding].fill(b'0');
        num_chars = strlen(s);
    }

    // add decimal point
    let point_position = num_chars - decimals;
    //shift content
    // by 1 space after point
    s.copy_within(
        point_position..point_position + decimals, //from: point to all the decimals
        point_position + 1,                        //to: just after point
    );
    s[point_position] = b'.';

    num_chars = strlen(s);

    return Ok(&mut s[..num_chars]);
}

#[cfg(test)]
mod tests {
    use super::intstr_to_fpstr_inplace;

    const SUITE: &[(&[u8], usize, &str)] = &[
        //NORMAL
        (b"1", 0, "1"),
        (b"123", 0, "123"),
        (b"123", 5, "0.00123"),
        (b"1234", 5, "0.01234"),
        (b"12345", 5, "0.12345"),
        (b"123456", 5, "1.23456"),
        (b"1234567", 5, "12.34567"),
        //EXTRA
        (b"12345", 2, "123.45"),
        (b"12", 0, "12"),
        (b"12", 1, "1.2"),
        (b"012", 1, "1.2"),
        (b"0012345", 3, "12.345"),
        (b"9", 6, "0.000009"),
        // TRIM LEADING
        (b"0", 0, "0"),
        (b"00", 0, "0"),
        (b"0000", 0, "0"),
        (b"00001", 0, "1"),
        (b"000011", 0, "11"),
        (b"10000", 0, "10000"),
        //EMPTY
        (b"", 0, "0"),
        (b"", 1, "0.0"),
        (b"", 2, "0.00"),
        (b"", 5, "0.00000"),
        (b"", 10, "0.0000000000"),
    ];

    #[test]
    fn intstr_to_fpstr_inplace_test() {
        for &(input, decimals, expected_output) in SUITE.iter() {
            std::dbg!(
                "SUITE:",
                (
                    core::str::from_utf8(&input).unwrap(),
                    decimals,
                    expected_output
                )
            );

            let mut input = std::vec::Vec::from(input);
            input.resize(input.len() + decimals + 2, 0);

            let out = intstr_to_fpstr_inplace(&mut input, decimals).unwrap();
            let out = core::str::from_utf8(out).unwrap();

            assert_eq!(out, expected_output)
        }
    }
}
