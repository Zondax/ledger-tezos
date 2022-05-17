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
pub(self) use crate::raw::{cx_hash_t, cx_md_t};

pub mod blake2b;
pub use blake2b::Blake2b;

pub mod sha256;
pub use sha256::Sha256;

pub mod sha512;
pub use sha512::Sha512;
///Perform a hash computation
///
/// if write_out is true then `out` must be of the necessary size
///
/// Abstracts away nanos or nanox implementations
#[inline(never)]
pub(self) fn cx_hash(
    hash: &mut cx_hash_t,
    input: &[u8],
    out: Option<&mut [u8]>,
) -> Result<(), Error> {
    zemu_sys::zemu_log_stack("cx_hash\x00");
    let (out, out_len, write_out): (*mut u8, u32, bool) = match out {
        Some(out) => (out.as_mut_ptr(), out.len() as u32, true),
        None => (std::ptr::null_mut(), 0, false),
    };

    cfg_if! {
        if #[cfg(bolos_sdk)] {
            match unsafe { crate::raw::cx_hash_no_throw(
                hash as *mut _,
                write_out as u8 as _,
                input.as_ptr() as *const _,
                input.len() as u32 as _,
                out as *mut _,
                out_len as _,
            )} {
                0 => Ok(()),
                err => Err(err.into())
            }
        } else {
            unimplemented!("cx_hash called in not bolos")
        }
    }
}

mod sealed {
    //This is intentionally private since we want only _our_ hashes to be able to implement it
    pub trait CxHash<const S: usize>: Sized {
        fn cx_init_hasher() -> Result<Self, super::Error>;

        fn cx_init_hasher_gce(loc: &mut core::mem::MaybeUninit<Self>) -> Result<(), super::Error>;

        fn cx_reset(&mut self) -> Result<(), super::Error>;

        fn cx_header(&mut self) -> &mut super::cx_hash_t;

        fn cx_id() -> super::cx_md_t;
    }
}

pub(self) use sealed::CxHash;

pub use bolos_common::hash::{Hasher, HasherId};

macro_rules! impl_hasher {
    (@__IMPL $ty:ty, $s:tt) => {
        type Error = Error;

        #[inline(never)]
        fn update(&mut self, input: &[u8]) -> Result<(), Self::Error> {
            cx_hash(self.cx_header(), input, None)
        }

        #[inline(never)]
        fn finalize_dirty_into(&mut self, out: &mut [u8; $s]) -> Result<(), Self::Error> {
            cx_hash(self.cx_header(), &[], Some(&mut out[..]))?;

            Ok(())
        }

        #[inline(never)]
        fn finalize_into(mut self, out: &mut [u8; $s]) -> Result<(), Self::Error> {
            cx_hash(self.cx_header(), &[], Some(out))?;

            Ok(())
        }

        #[inline(never)]
        fn reset(&mut self) -> Result<(), Self::Error> {
            self.cx_reset()
        }

        #[inline(never)]
        fn digest_into(input: &[u8], out: &mut [u8; $s]) -> Result<(), Self::Error> {
            zemu_sys::zemu_log_stack("Hasher::digest_into\x00");

            let mut hasher = core::mem::MaybeUninit::<Self>::uninit();
            Self::new_gce(&mut hasher)?;

            //Safety: this has just been initialized
            let mut hasher = unsafe { hasher.assume_init() };

            cx_hash(hasher.cx_header(), input, Some(&mut out[..]))?;

            Ok(())
        }
    };
    (@__IMPLID $ty:ty, $s:tt) => {
        type Id = u8;

        fn id() -> Self::Id {
            Self::cx_id() as Self::Id
        }
    };
    (@GENERIC $s:ident, $ty:ty) => {
        impl<const $s: usize> Hasher<S> for $ty
        where
            Self: CxHash<$s>,
        {
            impl_hasher! {@__IMPL $ty, $s}
        }

        impl<const $s: usize> HasherId for $ty
        where
            Self: CxHash<$s>,
        {
            impl_hasher! {@__IMPLID $ty, $s}
        }
    };
    (@FIXED $sz:expr, $ty:ty) => {
        impl Hasher<$sz> for $ty
        where
            Self: CxHash<$sz>,
        {
            impl_hasher! {@__IMPL $ty, $sz}
        }

        impl HasherId for $ty
        where
            Self: CxHash<$sz>,
        {
            impl_hasher! {@__IMPLID $ty, $sz}
        }
    };
}

impl_hasher! {@FIXED 32, Sha256}
impl_hasher! {@GENERIC S, Blake2b<S>}
impl_hasher! {@FIXED 64, Sha512}
