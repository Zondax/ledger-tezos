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
use arrayref::array_ref;
use nom::{bytes::complete::take, number::complete::le_u8, sequence::tuple, IResult};
use zemu_sys::ViewError;

use crate::{crypto::Curve, handlers::parser_common::ParserError};

pub mod operations;

#[cfg(feature = "baking")]
pub mod baking;

///This trait defines the interface useful in the UI context
/// so that all the different OperationTypes or other items can handle their own UI
pub trait DisplayableItem {
    /// Returns the number of items to display
    fn num_items(&self) -> usize;

    /// This is invoked when a given page is to be displayed
    ///
    /// `item_n` is the item of the operation to display;
    /// guarantee: 0 <= item_n < self.num_items()
    /// `title` is the title of the item
    /// `message` is the contents of the item
    /// `page` is what page we are supposed to display, this is used to split big messages
    ///
    /// returns the total number of pages on success
    ///
    /// It's a good idea to always put `#[inline(never)]` on top of this
    /// function's implementation
    //#[inline(never)]
    fn render_item(
        &self,
        item_n: u8,
        title: &mut [u8],
        message: &mut [u8],
        page: u8,
    ) -> Result<u8, ViewError>;
}

//legacy app stored in a uint64 always, we have `read_as`
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Zarith<'b> {
    bytes: &'b [u8],
    is_negative: Option<bool>,
}

impl<'b> Zarith<'b> {
    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    #[cfg(not(test))]
    pub fn from_bytes(input: &'b [u8], want_sign: bool) -> IResult<&[u8], Self, ParserError> {
        //keep taking bytes while the MSB is 1
        let (_, bytes) = nom::bytes::complete::take_till(|byte| byte & 0x80 == 0)(input)?;

        //take bytes + 1 since we miss the last byte with `take_till`
        let (rem, bytes) = take(bytes.len() + 1)(input)?;

        let is_negative = if want_sign {
            //if the second bit of the first byte is set, then it's negative
            Some(bytes[0] & 0x40 != 0)
        } else {
            None
        };

        Ok((rem, Self { bytes, is_negative }))
    }

    pub fn is_negative(&self) -> Option<bool> {
        self.is_negative
    }

    /// Attempts to read the zarith number in an N
    ///
    /// The first value of the tuple is true when the number is negative
    pub fn read_as<N>(&self) -> Option<(bool, N)>
    where
        //restrict so only integers can be used as containers to read the number into
        N: core::ops::BitOrAssign + core::ops::Shl<usize, Output = N> + From<u8> + Default,
    {
        let mut out = Default::default();
        let mut shift = 0;

        let n_bits = core::mem::size_of::<N>() * 8;

        let mut is_first = true;
        for b in self.bytes.iter() {
            let (mask, shift_add) = if is_first && self.is_negative.is_some() {
                is_first = false;
                //if this is the first byte and we did care for negative
                //then mask off the first and second MSbit
                // and signal to add 6 bits to the shift
                (!0b1100_0000, 6)
            } else {
                //only mask off the MSB
                // and signal to add 7 bits to the shift
                (!0x80, 7)
            };

            if shift + shift_add > n_bits {
                //we won't be able to fit
                // so we return None
                return None;
            }

            //mask the byte, removing the bits we don't want
            // even tho we are gonna override them the next loop...
            // and then shift that result by the number if bits written so far
            out |= N::from(b & mask) << shift;

            //increment the number of bits added to the number
            shift += shift_add;
        }

        Some((self.is_negative.unwrap_or_default(), out))
    }
}

#[cfg(test)]
impl<'b> Zarith<'b> {
    #[cfg(test)]
    pub fn from_bytes(input: &'b [u8], want_sign: bool) -> IResult<&[u8], Self, ParserError> {
        use nom::{dbg_basic, take, take_till};
        use std::println;

        //keep taking bytes while the MSB is 1
        let (_, bytes) = dbg_basic!(input, take_till!(|byte| byte & 0x80 == 0))?;

        //take bytes + 1 since we miss the last byte with `take_till`
        let (rem, bytes) = dbg_basic!(input, take!(bytes.len() + 1))?;

        let is_negative = if want_sign {
            //if the second bit of the first byte is set, then it's negative
            Some(bytes[0] & 0x40 != 0)
        } else {
            None
        };

        Ok((rem, Self { bytes, is_negative }))
    }

    pub fn is(&self, json: &serde_json::Value) {
        let num: f64 = json
            .as_str()
            .unwrap_or_else(|| panic!("given json for zarith was not a string; found={}", json))
            .parse()
            .unwrap_or_else(|e| {
                panic!(
                    "given json for zarith couldn't be parsed to f64; json={}; err: {:?}",
                    json, e
                )
            });

        if let Some(neg) = self.is_negative() {
            assert_eq!(neg, num < 0.0)
        }

        let (neg, z) = self.read_as::<u32>().expect("zarith didn't fit in u32");
        let mut z = z as f64;
        if neg {
            z = z.copysign(-0.0);
        }

        //we can't check equality for floating point numbers
        // but we can check their different is smaller than an EPSILON
        assert!((z - num).abs() < f64::EPSILON);
    }
}

pub fn public_key_hash(input: &[u8]) -> IResult<&[u8], (Curve, &[u8; 20]), ParserError> {
    let (rem, (crv, hash)) = tuple((le_u8, take(20usize)))(input)?;

    let crv = match crv {
        0x00 => Curve::Bip32Ed25519,
        0x01 => Curve::Secp256K1,
        0x02 => Curve::Secp256R1,
        _ => return Err(ParserError::parser_invalid_pubkey_encoding.into()),
    };

    let out = array_ref!(hash, 0, 20);

    Ok((rem, (crv, out)))
}

pub fn public_key(input: &[u8]) -> IResult<&[u8], (Curve, &[u8]), ParserError> {
    let (rem, crv) = le_u8(input)?;

    let (crv, take_pk) = match crv {
        0x00 => (Curve::Bip32Ed25519, take(32usize)),
        0x01 => (Curve::Secp256K1, take(33usize)),
        0x02 => (Curve::Secp256R1, take(33usize)),
        _ => return Err(ParserError::parser_invalid_pubkey_encoding.into()),
    };

    let (rem, pk) = take_pk(rem)?;

    Ok((rem, (crv, pk)))
}

fn boolean(input: &[u8]) -> IResult<&[u8], bool, ParserError> {
    let (rem, b) = le_u8(input)?;

    Ok((rem, b == 255))
}

// Previously called magic byte
#[repr(u8)]
#[derive(Debug, Clone, PartialEq, Copy)]
pub enum Preemble {
    ///Data is expected to be a block
    Block = 0x01,

    ///Data is expected to be an endorsement
    Endorsement = 0x02,

    ///Data is expected to be an operation
    Operation = 0x03,

    ///Used in the past but current meaning/usage unknown
    TBD = 0x04,

    ///Data is expected to be encoded michelson
    Michelson = 0x05,
}

impl Preemble {
    pub fn from_bytes(input: &[u8]) -> IResult<&[u8], Self, ParserError> {
        let (rem, preemble) = le_u8(input)?;
        match preemble {
            0x01 => Ok((rem, Self::Block)),
            0x02 => Ok((rem, Self::Endorsement)),
            0x03 => Ok((rem, Self::Operation)),
            0x04 => Ok((rem, Self::TBD)),
            0x05 => Ok((rem, Self::Michelson)),
            _ => Err(ParserError::parser_unexpected_type.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        crypto::Curve,
        handlers::public_key::Addr,
        parser::{boolean, public_key, public_key_hash},
    };

    use super::Zarith;

    #[test]
    fn pkh_ed() {
        const PKH_BASE58: &str = "tz1QZ6KY7d3BuZDT1d19dUxoQrtFPN2QJ3hn";
        const INPUT_HEX: &str = "0035e993d8c7aaa42b5e3ccd86a33390ececc73abd";

        let input = hex::decode(INPUT_HEX).expect("invalid input hex");

        let (rem, (crv, hash)) = public_key_hash(&input).expect("failed to parse input");

        assert_eq!(rem.len(), 0);
        assert_eq!(crv, Curve::Bip32Ed25519);

        let addr = Addr::from_hash(hash, crv).unwrap();
        let (len, addr) = addr.base58();
        assert_eq!(&addr[..len], PKH_BASE58.as_bytes());
    }

    #[test]
    #[should_panic(expected = "failed to parse pkh input")]
    fn pkh_ed_eof() {
        const INPUT_HEX: &str = "0035e993d8c7aaa42b5e3ccd86a333";

        let input = hex::decode(INPUT_HEX).expect("invalid input hex");

        public_key_hash(&input).expect("failed to parse pkh input");
    }

    #[test]
    fn pk_ed() {
        const INPUT_HEX: &str =
            "00ebcf82872f4942052704e95dc4bfa0538503dbece27414a39b6650bcecbff896";
        const PK_BASE58: &str = "edpkvS5QFv7KRGfa3b87gg9DBpxSm3NpSwnjhUjNBQrRUUR66F7C9g";

        let input = hex::decode(INPUT_HEX).expect("invalid input hex");

        let (rem, (crv, pk)) = public_key(&input).expect("failed to parse input");

        assert_eq!(rem.len(), 0);
        assert_eq!(crv, Curve::Bip32Ed25519);

        let mut vpk = crate::constants::tzprefix::EDPK.to_vec();
        vpk.extend_from_slice(pk);

        let pk = bs58::encode(vpk).with_check().into_string();
        assert_eq!(pk.as_str(), PK_BASE58);
    }

    #[test]
    #[should_panic(expected = "failed to parse pk input")]
    fn pk_ed_eof() {
        const INPUT_HEX: &str = "00ebcf82872f4942052704e95dc4bfa0538503dbece27414a39b";

        let input = hex::decode(INPUT_HEX).expect("invalid input hex");

        public_key(&input).expect("failed to parse pk input");
    }

    #[test]
    fn parse_boolean() {
        assert!(boolean(&[255]).expect("invalid input").1);
        assert!(!boolean(&[0]).expect("invalid input").1);
        assert!(!boolean(&[123]).expect("invalid input").1);
    }

    #[test]
    fn zarith() {
        //should ignore last byte
        let end_early = &[0b1000_0001, 0x11, 0x33][..];

        let (_, num) = Zarith::from_bytes(end_early, false).expect("invalid input");
        assert_eq!(num.len(), 2);
        assert_eq!(num.is_negative(), None);
        let num = num.read_as::<usize>().expect("didn't fit in usize").1;
        assert_eq!(num, 0x881);

        //should get a single byte
        let single_byte = &[0x0a][..];

        let (_, num) = Zarith::from_bytes(single_byte, false).expect("invalid input");
        assert_eq!(num.len(), 1);
        assert_eq!(num.is_negative(), None);

        let num = num.read_as::<usize>().expect("didn't fit in usize").1;
        assert_eq!(num, 0x0a);

        //should get a bunch of bytes
        let multi_byte = &[0x8a, 0x90, 0xf2, 0xe4, 0x88, 0x00][..];

        let (_, num) = Zarith::from_bytes(multi_byte, false).expect("invalid input");
        assert_eq!(num.len(), 6);
        assert_eq!(num.is_negative(), None);

        let num = num.read_as::<u64>().expect("didn't fit in u64").1;
        assert_eq!(num, 0x8C9C880A);

        //should be considered negative
        let negative = &[0b1100_0011, 0x23][..];

        let (_, num) = Zarith::from_bytes(negative, true).expect("invalid input");
        assert_eq!(num.len(), 2);
        assert_eq!(num.is_negative(), Some(true));

        let (neg, num) = num.read_as::<u64>().expect("didn't fit in u64");
        assert!(neg);
        assert_eq!(num, 0x8C3);

        //should be considered positive
        let positive = &[0b1000_0011, 0x23][..];

        let (_, num) = Zarith::from_bytes(positive, true).expect("invalid input");
        assert_eq!(num.len(), 2);
        assert_eq!(num.is_negative(), Some(false));

        let (neg, num) = num.read_as::<u64>().expect("didn't fit in u64");
        assert!(!neg);
        assert_eq!(num, 0x8C3);
    }
}

#[cfg(test)]
mod integration_tests;
