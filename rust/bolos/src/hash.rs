use crate::{errors::catch, raw::cx_hash, Error};

pub mod blake2b;
pub use blake2b::Blake2b;

pub mod sha256;
pub use sha256::Sha256;

mod sealed {
    //This is intentionally private since we want only _our_ hashes to be able to implement it
    pub trait CxHash<const S: usize>: Sized {
        fn cx_header(&mut self) -> &mut crate::raw::cx_hash_t;
    }
}

pub(self) use sealed::CxHash;

pub trait Hasher<const S: usize>: CxHash<S> {
    fn update(&mut self, input: &[u8]) -> Result<(), Error> {
        let might_throw = || unsafe {
            cx_hash(
                self.cx_header() as *mut _,
                false as u8 as _,
                &input[0] as *const u8 as *const _,
                input.len() as u32 as _,
                core::ptr::null_mut(),
                0u32 as _,
            );
        };

        catch(might_throw)?;

        Ok(())
    }

    fn finalize(mut self) -> Result<[u8; S], Error> {
        let mut out = [0; S];

        let might_throw = || unsafe {
            cx_hash(
                self.cx_header() as *mut _,
                true as u8 as _,
                core::ptr::null(),
                0u32 as _,
                &mut out[0] as *mut u8 as *mut _,
                out.len() as u32 as _,
            )
        };

        catch(might_throw)?;

        Ok(out)
    }
}

impl<H: CxHash<S>, const S: usize> Hasher<S> for H {}
