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
use std::mem::MaybeUninit;

use hmac::{crypto_mac::InvalidKeyLength, Hmac, Mac, NewMac};
use sha2::Sha256;

type Error = InvalidKeyLength;

pub struct Sha256HMAC(Hmac<Sha256>);

impl Sha256HMAC {
    pub fn new(key: &[u8]) -> Result<Self, Error> {
        let mut loc = MaybeUninit::uninit();

        Self::new_gce(&mut loc, key).map(|_| unsafe { loc.assume_init() })
    }

    pub fn new_gce(loc: &mut MaybeUninit<Self>, key: &[u8]) -> Result<(), Error> {
        *loc = MaybeUninit::new(Self(Hmac::new_from_slice(key)?));

        Ok(())
    }

    pub fn update(&mut self, input: &[u8]) -> Result<(), Error> {
        self.0.update(input);

        Ok(())
    }

    pub fn finalize_hmac_into(self, out: &mut [u8; 32]) -> Result<(), Error> {
        out.copy_from_slice(self.0.finalize().into_bytes().as_ref());
        Ok(())
    }

    pub fn finalize_hmac(#[allow(unused_mut)] mut self) -> Result<[u8; 32], Error> {
        let mut out = [0; 32];

        self.finalize_hmac_into(&mut out)?;

        Ok(out)
    }
}
