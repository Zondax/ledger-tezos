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

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl AsRef<[u8]> for PublicKey {
    fn as_ref(&self) -> &[u8] {
        todo!("asref ecfp256 pubkey")
    }
}

pub struct SecretKey<const B: usize> {
    curve: Curve,
}

impl<const B: usize> SecretKey<B> {
    pub const fn new(_mode: Mode, curve: Curve, _path: BIP32Path<B>) -> Self {
        Self { curve }
    }

    pub const fn curve(&self) -> Curve {
        self.curve
    }

    pub fn public(&self) -> Result<PublicKey, Error> {
        todo!("secret to public")
    }

    pub fn sign<H>(&self, _data: &[u8], _out: &mut [u8]) -> Result<usize, Error>
    where
        H: HasherId,
        H::Id: Into<u8>,
    {
        todo!("sign ecfp256")
    }
}
