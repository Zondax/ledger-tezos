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
use std::{convert::Infallible, mem::MaybeUninit};

pub struct Sha256HMAC {}

impl Sha256HMAC {
    pub fn new(key: &[u8]) -> Result<Self, Infallible> {
        let mut loc = MaybeUninit::uninit();

        Self::new_gce(&mut loc, key).map(|_| unsafe { loc.assume_init() })
    }

    pub fn new_gce(_loc: &mut MaybeUninit<Self>, _key: &[u8]) -> Result<(), Infallible> {
        todo!("sha256 hmac new gce")
    }

    pub fn update(&mut self, _input: &[u8]) -> Result<(), Infallible> {
        todo!("sha256 hmac update")
    }

    pub fn finalize_hmac_into(
        #[allow(unused_mut)] mut self,
        _out: &mut [u8; 32],
    ) -> Result<(), Infallible> {
        todo!("sha256 hmac finalize into")
    }

    pub fn finalize_hmac(#[allow(unused_mut)] mut self) -> Result<[u8; 32], Infallible> {
        let mut out = [0; 32];

        self.finalize_hmac_into(&mut out)?;

        Ok(out)
    }
}
