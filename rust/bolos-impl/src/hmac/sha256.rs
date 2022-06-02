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
#![allow(unused_imports)]

use crate::{
    errors::catch,
    raw::{cx_hmac_sha256_t, cx_hmac_t},
    Error,
};

use core::{mem::MaybeUninit, ptr::addr_of_mut};

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

    pub fn new_gce(loc: &mut MaybeUninit<Self>, key: &[u8]) -> Result<(), Error> {
        let state = unsafe { addr_of_mut!((*loc.as_mut_ptr()).state) };

        Self::init_state(state, key)
    }

    fn init_state(state: *mut cx_hmac_sha256_t, key: &[u8]) -> Result<(), Error> {
        cfg_if! {
            if #[cfg(bolos_sdk)] {
                match unsafe { crate::raw::cx_hmac_sha256_init_no_throw(
                    state as *mut _,
                    key.as_ptr() as *const _,
                    key.len() as u32 as _
                )} {
                    0 => {}
                    err => return Err(err.into()),
                }
            } else {
                unimplemented!("hmac sha256 init called in non-bolos")
            }
        }

        Ok(())
    }

    fn super_state(&mut self) -> &mut cx_hmac_t {
        //Safety: this ok since it's basically a downcast to a super class, in C
        // as a matter of fact, in the old sdk it's just a typedef
        unsafe { core::mem::transmute(&mut self.state) }
    }

    /// Add data to hmac but don't finalize it
    pub fn update(&mut self, input: &[u8]) -> Result<(), Error> {
        super::cx_hmac(self.super_state(), false, input, None)
    }

    pub fn finalize_hmac_into(mut self, out: &mut [u8; 32]) -> Result<(), Error> {
        super::cx_hmac(self.super_state(), false, &[], Some(&mut out[..]))
    }

    pub fn finalize_hmac(self) -> Result<[u8; 32], Error> {
        let mut out = [0; 32];

        self.finalize_hmac_into(&mut out)?;

        Ok(out)
    }
}
