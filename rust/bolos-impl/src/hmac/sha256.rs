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

use crate::{
    errors::catch,
    hash::{HasherId, Sha256},
    raw::{cx_hmac_sha256_t, cx_hmac_t},
    Error,
};

pub struct Sha256HMAC {
    state: cx_hmac_sha256_t,
}

impl Sha256HMAC {
    pub fn new(key: &[u8]) -> Result<Self, Error> {
        let mut this = Self {
            state: Default::default(),
        };

        Self::init_state(&mut this.state, key)?;

        Ok(this)
    }

    fn init_state(state: &mut cx_hmac_sha256_t, key: &[u8]) -> Result<(), Error> {
        cfg_if! {
            if #[cfg(nanox)] {
                let might_throw = || unsafe { crate::raw::cx_hmac_sha256_init(
                    state as *mut _,
                    key.as_ptr() as *const _,
                    key.len() as u32 as _);
                };

                catch(might_throw)?;
            } else if #[cfg(nanos)] {
                let id = Sha256::id() as u32;

                match unsafe { crate::raw::cx_hmac_sha256_init_no_throw(
                    state as *mut _,
                    key.as_ptr() as *const _,
                    key.len() as u32 as _
                )} {
                    //check that the return matches with the hash id
                    r if r == id => {},
                    err => return Err(err.into()),
                }
            } else {
                todo!("hmac sha256 init called in non-bolos")
            }
        }

        Ok(())
    }

    pub fn finalize_hmac_into(mut self, input: &[u8], out: &mut [u8; 32]) -> Result<(), Error> {
        //Safety: this ok since it's basically a downcast to a super class, in C
        // as a matter of fact, in the old sdk it's just a typedef
        let state: &mut cx_hmac_t = unsafe { core::mem::transmute(&mut self.state) };

        super::cx_hmac(state, false, input, Some(&mut out[..]))
    }

    pub fn finalize_hmac(self, input: &[u8]) -> Result<[u8; 32], Error> {
        let mut out = [0; 32];

        self.finalize_hmac_into(input, &mut out)?;

        Ok(out)
    }
}
