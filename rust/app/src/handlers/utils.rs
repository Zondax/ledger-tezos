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
use zemu_sys::ViewError;

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

    let mut digest = Sha256::new()?;
    for p in pieces {
        digest.update(p)?;
    }

    let x1 = digest.finalize_dirty()?;
    digest.reset()?;
    digest.update(&x1[..])?;

    let complete_digest = digest.finalize()?;

    out.copy_from_slice(&complete_digest[..4]);

    Ok(())
}

pub fn handle_ui_message(item: &[u8], out: &mut [u8], page: u8) -> Result<u8, ViewError> {
    let m_len = out.len() - 1; //null byte terminator
    if m_len <= item.len() {
        let chunk = item
            .chunks(m_len / 2) //divide in non-overlapping chunks
            .nth(page as usize) //get the nth chunk
            .ok_or(ViewError::Unknown)?;

        out[..chunk.len()].copy_from_slice(chunk);
        out[chunk.len() * 2] = 0; //null terminate

        let n_pages = item.len() / m_len;
        Ok(1 + n_pages as u8)
    } else {
        out[..item.len()].copy_from_slice(item);
        out[item.len()] = 0; //null terminate
        Ok(1)
    }
}
