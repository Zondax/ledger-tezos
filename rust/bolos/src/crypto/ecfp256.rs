use zeroize::Zeroizing;

use super::{Curve, bip32::BIP32Path};

pub struct PublicKey {
    curve: Curve,
    len: usize,
    w: [u8; 65],
}

pub struct SecretKey {
    curve: Curve,
    len: usize,
    d: Zeroizing<[u8; 32]>,
}

pub struct Keypair {
    pub public: PublicKey,
    pub secret: SecretKey,
}

#[derive(Debug)]
pub enum GenerateError {}

impl Keypair {
    pub fn generate(curve: Curve, path: &BIP32Path) -> Result<Self, GenerateError> {
        todo!("generate keypair")
    }
}

#[cfg(bolos_sdk)]
mod bindings {

}
