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
use std::convert::Infallible;

pub struct Sha256HMAC {}

impl Sha256HMAC {
    pub fn new(_key: &[u8]) -> Result<Self, Infallible> {
        todo!("sha256 hmac new")
    }

    pub fn finalize_hmac_into(
        #[allow(unused_mut)] mut self,
        _input: &[u8],
        _out: &mut [u8; 32],
    ) -> Result<(), Infallible> {
        todo!("sha256 hmac finalize into")
    }

    pub fn finalize_hmac(
        #[allow(unused_mut)] mut self,
        input: &[u8],
    ) -> Result<[u8; 32], Infallible> {
        let mut out = [0; 32];

        self.finalize_hmac_into(input, &mut out)?;

        Ok(out)
    }
}
