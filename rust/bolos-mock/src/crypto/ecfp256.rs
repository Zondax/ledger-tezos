use bolos_common::hash::HasherId;

use crate::Error;

use super::{bip32::BIP32Path, Curve, Mode};

#[derive(Debug, Clone, Copy)]
pub struct PublicKey {
    curve: Curve,
}

impl PublicKey {
    pub fn compress(&mut self) -> Result<(), Error> {
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

impl SecretKey {
    pub fn sign<H>(&mut self, _data: &[u8], _out: &mut [u8]) -> Result<(), Error>
    where
        H: HasherId,
        H::Id: Into<u8>,
    {
        todo!("sign ecfp256")
    }
}

pub struct Keypair {
    pub public: PublicKey,
    pub secret: SecretKey,
}

impl Keypair {
    pub fn generate<const B: usize>(
        _mode: Mode,
        _curve: Curve,
        _path: &BIP32Path<B>,
    ) -> Result<Self, Error> {
        todo!("generate keypair ecfp256")
    }

    pub fn public(&self) -> &PublicKey {
        &self.public
    }
}
