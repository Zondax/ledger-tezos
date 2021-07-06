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
use std::convert::TryFrom;

use crate::{
    constants::{EDWARDS_SIGN_BUFFER_MIN_LENGTH, SECP256_SIGN_BUFFER_MIN_LENGTH},
    sys,
};
use bolos::hash::{Blake2b, Sha256};
use sys::{crypto::bip32::BIP32Path, errors::Error, hash::Hasher};

#[derive(Debug, Clone, Copy)]
pub struct PublicKey(pub(crate) sys::crypto::ecfp256::PublicKey);

impl PublicKey {
    pub fn compress(&mut self) -> Result<(), Error> {
        self.0.compress()
    }

    #[inline(never)]
    pub fn hash(&self, out: &mut [u8; 20]) -> Result<(), Error> {
        sys::zemu_log_stack("PublicKey::hash\x00");

        let mut hasher = Blake2b::new()?;

        match self.curve() {
            Curve::Bip32Ed25519 | Curve::Ed25519 => {
                let bytes = self.0.as_ref();
                let len = self.0.len();

                //skip the first byte when hashing
                hasher.update(&bytes[1..len])?;
            }
            Curve::Secp256K1 | Curve::Secp256R1 => {
                let bytes = self.0.as_ref();

                //calculate a new first byte
                let first = 0x02 + (bytes[64] & 0x01);
                hasher.update(&[first])?;

                //we already hashed the first byte
                // so hash from the second to the 33rd (ignore the rest)
                hasher.update(&bytes[1..33])?;
            }
        }

        hasher.finalize_into(out)
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

#[derive(Debug, Clone, Copy, PartialEq)]
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

impl From<Curve> for u8 {
    fn from(from: Curve) -> Self {
        match from {
            Curve::Ed25519 => 0,
            Curve::Secp256K1 => 1,
            Curve::Secp256R1 => 2,
            Curve::Bip32Ed25519 => 3,
        }
    }
}

impl From<Curve> for sys::crypto::Curve {
    fn from(from: Curve) -> Self {
        match from {
            Curve::Ed25519 | Curve::Bip32Ed25519 => Self::Ed25519,
            Curve::Secp256K1 => Self::Secp256K1,
            Curve::Secp256R1 => Self::Secp256R1,
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
            #[allow(unreachable_patterns)]
            //this isn't actually unreachable because CCurve mock is just incomplete
            _ => Err(()),
        }
    }
}

pub struct SecretKey<const B: usize>(sys::crypto::ecfp256::SecretKey<B>);

pub enum SignError {
    BufferTooSmall,
    Sys(Error),
}

impl<const B: usize> SecretKey<B> {
    pub fn new(curve: Curve, path: BIP32Path<B>) -> Self {
        use sys::crypto::Mode;

        let mode = match curve {
            Curve::Ed25519 => Mode::Ed25519Slip10,
            _ => Mode::BIP32,
        };
        Self(sys::crypto::ecfp256::SecretKey::new(
            mode,
            curve.into(),
            path,
        ))
    }

    pub fn into_public(self) -> Result<PublicKey, Error> {
        self.0.public().map(PublicKey)
    }

    pub fn curve(&self) -> Curve {
        use std::convert::TryInto;
        //this unwrap is ok because the curve
        // can only be initialized by the library and not the user

        self.0.curve().try_into().unwrap()
    }

    pub fn sign(&self, data: &[u8], out: &mut [u8]) -> Result<usize, SignError> {
        match self.curve() {
            Curve::Ed25519 | Curve::Bip32Ed25519 if out.len() < EDWARDS_SIGN_BUFFER_MIN_LENGTH => {
                Err(SignError::BufferTooSmall)
            }
            Curve::Secp256K1 | Curve::Secp256R1 if out.len() < SECP256_SIGN_BUFFER_MIN_LENGTH => {
                Err(SignError::BufferTooSmall)
            }

            Curve::Ed25519 | Curve::Bip32Ed25519 | Curve::Secp256K1 | Curve::Secp256R1 => self
                .0
                .sign::<Sha256>(data, out) //pass Sha256 for the signature nonce hasher
                .map_err(SignError::Sys),
        }
    }
}

impl Curve {
    pub fn to_secret<const B: usize>(self, path: &BIP32Path<B>) -> SecretKey<B> {
        SecretKey::new(self, *path)
    }
}
