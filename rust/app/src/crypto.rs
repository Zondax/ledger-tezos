use std::convert::TryFrom;

use crate::sys::{self, crypto::bip32::BIP32Path};

#[derive(Debug, Clone, Copy)]
pub enum Curve {
    Ed25519,
    Secp256K1,
    Secp256R1,
    Bip32Ed25519,
}

impl TryFrom<u8> for Curve {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Ed25519),
            1 => Ok(Self::Secp256K1),
            2 => Ok(Self::Secp256R1),
            3 => Ok(Self::Bip32Ed25519),
            _ => Err(()),
        }
    }
}

impl Into<u8> for Curve {
    fn into(self) -> u8 {
        match self {
            Curve::Ed25519 => 0,
            Curve::Secp256K1 => 1,
            Curve::Secp256R1 => 2,
            Curve::Bip32Ed25519 => 3,
        }
    }
}

impl Into<sys::crypto::Curve> for &Curve {
    fn into(self) -> sys::crypto::Curve {
        use sys::crypto::Curve as CCurve;

        match self {
            Curve::Ed25519 | Curve::Bip32Ed25519 => CCurve::Ed25519,
            Curve::Secp256K1 => CCurve::Secp256K1,
            Curve::Secp256R1 => CCurve::Secp256R1,
        }
    }
}

impl Curve {
    pub fn gen_keypair(
        &self,
        path: &BIP32Path,
    ) -> Result<sys::crypto::ecfp256::Keypair, GenerateKeyError> {
        let pair = sys::crypto::ecfp256::Keypair::generate(self.into(), path).unwrap();

        Ok(pair)
    }
}
