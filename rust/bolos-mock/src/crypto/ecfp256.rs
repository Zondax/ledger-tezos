use crate::Error;

use super::{bip32::BIP32Path, Curve, Mode};

#[derive(Debug, Clone, Copy)]
pub struct PublicKey {
    curve: Curve,
}

impl PublicKey {
    pub fn compress(&self) -> Result<Self, Error> {
        todo!("compress ecfp256 pubkey")
    }

    pub fn curve(&self) -> Curve {
        self.curve
    }

    pub fn len(&self) -> usize {
        todo!("len ecfp256 pubkey")
    }
}

impl AsRef<[u8]> for PublicKey {
    fn as_ref(&self) -> &[u8] {
        todo!("asref ecfp256 pubkey")
    }
}

pub struct SecretKey {}

pub struct Keypair {
    pub public: PublicKey,
    pub secret: SecretKey,
}

impl Keypair {
    pub fn generate(_mode: Mode, _curve: Curve, _path: &BIP32Path) -> Result<Self, Error> {
        todo!("generate keypair ecfp256")
    }
}
