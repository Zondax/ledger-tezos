pub use bolos_common::bip32;

use std::convert::TryFrom;

#[derive(Debug, Clone, Copy)]
pub enum Curve {
    Secp256K1,
    Secp256R1,

    Ed25519,
}

impl TryFrom<u8> for Curve {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value as u32 {
            1 => Ok(Self::Secp256K1),
            2 => Ok(Self::Secp256R1),
            3 => Ok(Self::Ed25519),

            _ => Err(()),
        }
    }
}

impl Into<u8> for Curve {
    fn into(self) -> u8 {
        let n = match self {
            Curve::Secp256K1 => 1,
            Curve::Secp256R1 => 2,
            Curve::Ed25519 => 3,
        };
        n as u8
    }
}

impl Curve {
    pub fn is_weirstrass(&self) -> bool {
        match self {
            Self::Secp256K1 | Self::Secp256R1 => true,
            _ => false,
        }
    }

    pub fn is_twisted_edward(&self) -> bool {
        match self {
            Self::Ed25519 => true,
            _ => false,
        }
    }

    pub fn is_montgomery(&self) -> bool {
        match self {
            _ => false,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Mode {
    BIP32,
    Ed25519Slip10,
    // Slip21,
}

impl TryFrom<u8> for Mode {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value as u32 {
            0 => Ok(Self::BIP32),
            1 => Ok(Self::Ed25519Slip10),
            // 2 => Ok(Self::Slip21),
            _ => Err(()),
        }
    }
}

impl Into<u8> for Mode {
    fn into(self) -> u8 {
        let n = match self {
            Mode::BIP32 => 0,
            Mode::Ed25519Slip10 => 1,
            // Mode::Slip21 => HDW_SLIP21,
        };

        n as u8
    }
}

impl Default for Mode {
    fn default() -> Self {
        Self::BIP32
    }
}

pub mod ecfp256;
