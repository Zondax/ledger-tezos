use std::convert::TryFrom;

use crate::sys;
use sys::{crypto::bip32::BIP32Path, errors::Error, hash::Hasher};

#[derive(Debug, Clone, Copy)]
pub struct PublicKey(pub(crate) sys::crypto::ecfp256::PublicKey);

impl PublicKey {
    pub fn compress(&mut self) -> Result<(), Error> {
        self.0.compress()
    }

    #[inline(never)]
    pub fn hash(&self) -> Result<[u8; 20], Error> {
        let mut key = [0; 65];

        //legacy/src/keys.c:118
        let len = {
            match self.curve() {
                Curve::Bip32Ed25519 | Curve::Ed25519 => {
                    let bytes = &self.0.as_ref();
                    let len = self.0.len();

                    let new_len = len - 1;
                    //copy all but the first byte
                    key[..new_len].copy_from_slice(&bytes[1..1 + new_len]);

                    new_len
                }
                Curve::Secp256K1 | Curve::Secp256R1 => {
                    let bytes = self.0.as_ref();

                    //copy only 33 bytes
                    key[..33].copy_from_slice(&bytes[..33]);

                    //and change a few things
                    key[0] = 0x02 + (bytes[64] & 0x01);

                    33
                }
            }
        };

        sys::hash::Blake2b::digest(&key[..len])
    }

    pub fn curve(&self) -> Curve {
        use std::convert::TryInto;
        //this unwrap is ok because the curve
        // can only be initialized by the library and not the user

        self.0.curve().try_into().unwrap()
    }
}

impl AsRef<[u8]> for PublicKey {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

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
            Self::Ed25519 => 0,
            Self::Secp256K1 => 1,
            Self::Secp256R1 => 2,
            Self::Bip32Ed25519 => 3,
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

impl TryFrom<sys::crypto::Curve> for Curve {
    type Error = ();

    fn try_from(ccrv: sys::crypto::Curve) -> Result<Self, Self::Error> {
        use sys::crypto::Curve as CCurve;

        match ccrv {
            CCurve::Ed25519 => Ok(Self::Bip32Ed25519),
            CCurve::Secp256K1 => Ok(Self::Secp256K1),
            CCurve::Secp256R1 => Ok(Self::Secp256R1),
            _ => Err(()),
        }
    }
}

pub struct Keypair {
    pub public: PublicKey,
    pub secret: sys::crypto::ecfp256::SecretKey,
}

impl Keypair {
    pub fn into_public(self) -> PublicKey {
        self.public
    }
}

impl Curve {
    pub fn gen_keypair(&self, path: &BIP32Path) -> Result<Keypair, Error> {
        use sys::crypto::Mode;

        let mode = match self {
            Self::Ed25519 => Mode::Ed25519Slip10,

            _ => Default::default(),
        };

        let kp = sys::crypto::ecfp256::Keypair::generate(mode, self.into(), path)?;
        Ok(Keypair {
            public: PublicKey(kp.public),
            secret: kp.secret,
        })
    }
}
