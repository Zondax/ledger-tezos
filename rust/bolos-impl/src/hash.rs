#![allow(unused_imports)]

use crate::errors::{catch, Error};
pub(self) use crate::raw::cx_hash_t;

pub mod blake2b;
pub use blake2b::Blake2b;

pub mod sha256;
pub use sha256::Sha256;

///Perform a hash computation
///
/// if write_out is true then `out` must be of the necessary size
///
/// Abstracts away nanos or nanox implementations
pub(self) fn cx_hash(
    hash: &mut cx_hash_t,
    input: &[u8],
    out: Option<&mut [u8]>,
) -> Result<(), Error> {
    let (out, out_len, write_out): (*mut u8, u32, bool) = match out {
        Some(out) => (out.as_mut_ptr(), out.len() as u32, true),
        None => (std::ptr::null_mut(), 0, false),
    };

    cfg_if! {
        if #[cfg(nanox)] {
            let might_throw = || unsafe { crate::raw::cx_hash(
                hash as *mut _,
                write_out as u8 as _,
                input.as_ptr() as *const _,
                input.len() as u32 as _,
                out as *mut _,
                out_len as _,
            )};

            catch(might_throw)?;
            Ok(())

        } else if #[cfg(nanos)] {
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
            todo!("cx_hash called in not bolos")
        }
    }
}

mod sealed {
    //This is intentionally private since we want only _our_ hashes to be able to implement it
    pub trait CxHash<const S: usize>: Sized {
        fn init_hasher() -> Result<Self, super::Error>;

        fn reset(&mut self) -> Result<(), super::Error>;

        fn cx_header(&mut self) -> &mut super::cx_hash_t;
    }
}

pub(self) use sealed::CxHash;

pub use bolos_common::hash::Hasher;

macro_rules! impl_hasher {
    (@__IMPL $ty:ty, $s:tt) => {
        type Error = Error;

        fn update(&mut self, input: &[u8]) -> Result<(), Self::Error> {
            cx_hash(self.cx_header(), input, None)
        }

        fn finalize_dirty(&mut self) -> Result<[u8; $s], Self::Error> {
            let mut out = [0; $s];

            cx_hash(self.cx_header(), &[], Some(&mut out[..]))?;
            Ok(out)
        }

        fn finalize(mut self) -> Result<[u8; $s], Self::Error> {
            self.finalize_dirty()
        }

        fn reset(&mut self) -> Result<(), Self::Error> {
            CxHash::reset(self)
        }

        #[inline(never)]
        fn digest(input: &[u8]) -> Result<[u8; $s], Self::Error> {
            let mut hasher = Self::init_hasher()?;

            let mut out = [0; $s];
            cx_hash(hasher.cx_header(), input, Some(&mut out[..]))?;

            Ok(out)
        }
    };
    (@GENERIC $s:ident, $ty:ty) => {
        impl<const $s: usize> Hasher<S> for $ty
        where
            Self: CxHash<$s>,
        {
            impl_hasher! {@__IMPL $ty, $s}
        }
    };
    (@FIXED $sz:expr, $ty:ty) => {
        impl Hasher<$sz> for $ty
        where
            Self: CxHash<$sz>,
        {
            impl_hasher! {@__IMPL $ty, $sz}
        }
    };
}

impl_hasher! {@FIXED 32, Sha256}
impl_hasher! {@GENERIC S, Blake2b<S>}
