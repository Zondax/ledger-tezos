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

use crate::errors::{catch, Error};
pub(self) use crate::raw::{cx_hmac_t, CX_NO_REINIT};

pub mod sha256;
pub use sha256::Sha256HMAC;

///Perform a hmac computation
///
///If `reinit` is set then the `hmac` contex
/// is reinitialized if the hmac is written to `out` (if `out` is `Some`)
///
/// `hmac` context is expected to be initialized
///
/// Abstracts away nanos or nanox implementations
pub(self) fn cx_hmac(
    hmac: &mut cx_hmac_t,
    reinit: bool,
    input: &[u8],
    out: Option<&mut [u8]>,
) -> Result<(), Error> {
    zemu_sys::zemu_log_stack("cx_hmac\x00");

    let (out, out_len, write_out): (*mut u8, u32, bool) = match out {
        Some(out) => (out.as_mut_ptr(), out.len() as u32, true),
        None => (std::ptr::null_mut(), 0, false),
    };

    let reinit = if !reinit { CX_NO_REINIT } else { 0 };
    let mode = reinit | write_out as u8 as u32;

    cfg_if! {
        if #[cfg(bolos_sdk)] {
            match unsafe { crate::raw::cx_hmac_no_throw(
                hmac as *mut _,
                mode as _,
                input.as_ptr() as *const _,
                input.len() as u32 as _,
                out as *mut _,
                out_len as _,
            )} {
                0 => Ok(()),
                err => Err(err.into())
            }
        } else {
            unimplemented!("cx_hmac called in not bolos")
        }
    }
}
