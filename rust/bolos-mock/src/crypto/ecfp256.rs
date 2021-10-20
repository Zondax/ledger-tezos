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
use bolos_common::hash::HasherId;
use core::mem::MaybeUninit;

use crate::Error;

use super::{bip32::BIP32Path, Curve, Mode};

#[derive(Clone, Copy)]
pub struct PublicKey {
    curve: Curve,
    len: usize,
    data: [u8; 65],
}

impl PublicKey {
    pub fn compress(&mut self) -> Result<(), Error> {
        match self.curve {
            Curve::Secp256K1 => {
                let point = k256::EncodedPoint::from_bytes(&self.data[..self.len]).unwrap();
                let compressed = point.compress();

                self.data[..33].copy_from_slice(compressed.as_ref());
                Ok(())
            }
            Curve::Secp256R1 => {
                let point = p256::EncodedPoint::from_bytes(&self.data[..self.len]).unwrap();
                let compressed = point.compress();

                self.data[..33].copy_from_slice(compressed.as_ref());
                Ok(())
            }
            _ => Ok(()),
        }
    }

    pub fn curve(&self) -> Curve {
        self.curve
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl AsRef<[u8]> for PublicKey {
    fn as_ref(&self) -> &[u8] {
        &self.data[..self.len]
    }
}

pub struct SecretKey<const B: usize> {
    curve: Curve,
    bytes: [u8; 32],
}

impl<const B: usize> SecretKey<B> {
    pub fn new(_: Mode, curve: Curve, _: BIP32Path<B>) -> Self {
        let bytes = match curve {
            Curve::Secp256K1 => {
                let secret = k256::ecdsa::SigningKey::random(&mut rand8::thread_rng());

                *secret.to_bytes().as_ref()
            }
            Curve::Secp256R1 => {
                let secret = p256::ecdsa::SigningKey::random(&mut rand8::thread_rng());

                *secret.to_bytes().as_ref()
            }
            Curve::Ed25519 => {
                let secret = ed25519_dalek::SecretKey::generate(&mut rand7::thread_rng());

                secret.to_bytes()
            }
        };

        Self { curve, bytes }
    }

    pub const fn curve(&self) -> Curve {
        self.curve
    }

    pub fn public(&self) -> Result<PublicKey, Error> {
        let (data, len) = match self.curve {
            Curve::Secp256K1 => {
                let secret = k256::ecdsa::SigningKey::from_bytes(&self.bytes[..]).unwrap();

                let public = secret.verifying_key();
                //this is already compressed
                let compressed_point = public.to_bytes();
                let uncompressed_point = k256::EncodedPoint::from_bytes(compressed_point)
                    .unwrap()
                    .decompress()
                    .unwrap();
                let uncompressed_point = uncompressed_point.as_ref();

                let mut bytes = [0; 65];
                bytes[..uncompressed_point.len()].copy_from_slice(uncompressed_point);

                (bytes, uncompressed_point.len())
            }
            Curve::Secp256R1 => {
                let secret = p256::ecdsa::SigningKey::from_bytes(&self.bytes[..]).unwrap();

                let public = secret.verifying_key();
                //when we encode we don't compress the point right away
                let uncompressed_point = public.to_encoded_point(false);
                let uncompressed_point = uncompressed_point.as_ref();

                let mut bytes = [0; 65];
                bytes[..uncompressed_point.len()].copy_from_slice(uncompressed_point);

                (bytes, uncompressed_point.len())
            }
            Curve::Ed25519 => {
                let secret = ed25519_dalek::SecretKey::from_bytes(&self.bytes[..]).unwrap();

                let public = ed25519_dalek::PublicKey::from(&secret);
                let mut bytes = [0; 65];
                bytes[..32].copy_from_slice(&public.as_bytes()[..]);

                (bytes, 32)
            }
        };

        Ok(PublicKey {
            curve: self.curve,
            data,
            len,
        })
    }

    pub fn public_into(&self, out: &mut MaybeUninit<PublicKey>) -> Result<(), Error> {
        let pk = self.public()?;

        *out = MaybeUninit::new(pk);

        Ok(())
    }

    pub fn sign<H>(&self, data: &[u8], out: &mut [u8]) -> Result<usize, Error>
    where
        H: HasherId,
        H::Id: Into<u8>,
    {
        match self.curve {
            Curve::Secp256K1 => {
                use k256::ecdsa::{signature::Signer, Signature};

                let secret = k256::ecdsa::SigningKey::from_bytes(&self.bytes[..]).unwrap();

                let sig: Signature = secret.sign(data);
                let sig = sig.as_ref();

                out[..sig.len()].copy_from_slice(sig);
                Ok(sig.len())
            }
            Curve::Secp256R1 => {
                use p256::ecdsa::signature::Signer;

                let secret = p256::ecdsa::SigningKey::from_bytes(&self.bytes[..]).unwrap();
                let sig = secret.sign(data);
                let sig = sig.as_ref();

                out[..sig.len()].copy_from_slice(sig);
                Ok(sig.len())
            }
            Curve::Ed25519 => {
                use ed25519_dalek::Signer;

                let secret = ed25519_dalek::SecretKey::from_bytes(&self.bytes[..]).unwrap();
                let public = ed25519_dalek::PublicKey::from(&secret);

                let keypair = ed25519_dalek::Keypair { secret, public };
                let sig = keypair.sign(data);

                out[..64].copy_from_slice(&sig.to_bytes()[..]);
                Ok(64)
            }
        }
    }
}
