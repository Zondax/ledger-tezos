use core::convert::TryFrom;

use crate::constants::ApduError;

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

impl Into<u8> for ZPacketType {
    fn into(self) -> u8 {
        self as _
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

impl Into<u8> for LegacyPacketType {
    fn into(self) -> u8 {
        self as _
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
