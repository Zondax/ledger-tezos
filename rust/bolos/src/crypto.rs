//! Module containing all crypto primitives and utilities
//! for rust ledger apps
//!
//! TODO: move utilities to separate crate

use std::convert::TryFrom;

pub mod bip32;

pub enum Curve {
    None,

    /* Secp.org */
    Secp256K1,
    Secp256R1,
    Secp384R1,
    Secp521R1,

    /* Brainpool */
    BrainPoolP256T1,
    BrainPoolP256R1,
    BrainPoolP320T1,
    BrainPoolP320R1,
    BrainPoolP384T1,
    BrainPoolP384R1,
    BrainPoolP512T1,
    BrainPoolP512R1,

    /* NIST P256 */
    Nistp256, //alias to Secp256R1
    Nistp384, //alias to Secp384R1
    Nistp521, //alias to Secp521R1

    /* ANSSI P256 */
    Frp256V1,

    /* Stark */
    Stark256,

    /* BLS */
    Bls12_381G1,

    /* Ed25519 */
    Ed25519,
    Ed448,

    /* Curve25519 */
    Curve25519,
    Curve448,
}

impl TryFrom<u8> for Curve {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::None),
            21 => Ok(Self::Secp256K1),
            22 => Ok(Self::Secp256R1),
            23 => Ok(Self::Secp384R1),
            24 => Ok(Self::Secp521R1),
            25 => Ok(Self::BrainPoolP256T1),
            26 => Ok(Self::BrainPoolP256R1),
            27 => Ok(Self::BrainPoolP320T1),
            28 => Ok(Self::BrainPoolP320R1),
            29 => Ok(Self::BrainPoolP384T1),
            30 => Ok(Self::BrainPoolP384R1),
            31 => Ok(Self::BrainPoolP512T1),
            32 => Ok(Self::BrainPoolP512R1),
            33 => Ok(Self::Frp256V1),
            34 => Ok(Self::Stark256),
            35 => Ok(Self::Bls12_381G1),

            41 => Ok(Self::Ed25519),
            42 => Ok(Self::Ed448),

            61 => Ok(Self::Curve25519),
            62 => Ok(Self::Curve448),
            _ => Err(()),
        }
    }
}

impl Into<u8> for Curve {
    fn into(self) -> u8 {
        match self {
            Curve::None => 0,
            Curve::Secp256K1 => 21,
            Curve::Secp256R1 | Curve::Nistp256 => 22,
            Curve::Secp384R1 | Curve::Nistp384 => 23,
            Curve::Secp521R1 | Curve::Nistp521 => 24,
            Curve::BrainPoolP256T1 => 25,
            Curve::BrainPoolP256R1 => 26,
            Curve::BrainPoolP320T1 => 27,
            Curve::BrainPoolP320R1 => 28,
            Curve::BrainPoolP384T1 => 29,
            Curve::BrainPoolP384R1 => 30,
            Curve::BrainPoolP512T1 => 31,
            Curve::BrainPoolP512R1 => 32,
            Curve::Frp256V1 => 33,
            Curve::Stark256 => 34,
            Curve::Bls12_381G1 => 35,
            Curve::Ed25519 => 41,
            Curve::Ed448 => 42,
            Curve::Curve25519 => 61,
            Curve::Curve448 => 62,
        }
    }
}

impl Curve {
    pub fn is_weirstrass(&self) -> bool {
        match self {
            Self::Secp256K1
            | Self::Secp256R1
            | Self::Secp384R1
            | Self::Secp521R1
            | Self::BrainPoolP256T1
            | Self::BrainPoolP256R1
            | Self::BrainPoolP320T1
            | Self::BrainPoolP320R1
            | Self::BrainPoolP384T1
            | Self::BrainPoolP384R1
            | Self::BrainPoolP512T1
            | Self::BrainPoolP512R1
            | Self::Nistp256
            | Self::Nistp384
            | Self::Nistp521
            | Self::Frp256V1
            | Self::Stark256
            | Self::Bls12_381G1 => true,
            _ => false,
        }
    }

    pub fn is_twisted_edward(&self) -> bool {
        match self {
            Self::Ed25519 | Self::Ed448 => true,
            _ => false,
        }
    }

    pub fn is_montgomery(&self) -> bool {
        match self {
            Self::Curve25519 | Self::Curve448 => true,
            _ => false,
        }
    }
}

pub mod ecfp256;
