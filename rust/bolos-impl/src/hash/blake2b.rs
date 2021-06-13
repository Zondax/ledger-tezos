#![allow(unused_imports)]

use crate::raw::{cx_blake2b_t, cx_hash_t, cx_md_t};
use crate::{errors::catch, Error};

use super::CxHash;

#[repr(transparent)]
pub struct Blake2b<const S: usize> {
    state: cx_blake2b_t,
}

impl<const S: usize> Blake2b<S> {
    #[inline(never)]
    pub fn new() -> Result<Self, Error> {
        zemu_sys::zemu_log_stack("Blake2b::new\x00");
        let mut this = Self {
            state: Default::default(),
        };

        Self::init_state(&mut this.state)?;

        Ok(this)
    }

    fn init_state(state: &mut cx_blake2b_t) -> Result<(), Error> {
        cfg_if! {
            if #[cfg(nanox)] {
                let might_throw = || unsafe {
                    crate::raw::cx_blake2b_init(state as *mut _, (S * 8) as u32);
                };

                catch(might_throw)?;
            } else if #[cfg(nanos)] {
                let r = unsafe {
                    crate::raw::cx_blake2b_init_no_throw(
                        state as *mut _,
                        (S * 8) as u32
                    )
                };

                match r {
                    0 => {},
                    err => return Err(err.into()),
                }
            } else {
                todo!("blake2b init called in non bolos")
            }
        }

        Ok(())
    }
}

impl<const S: usize> CxHash<S> for Blake2b<S> {
    fn cx_init_hasher() -> Result<Self, Error> {
        Self::new()
    }

    fn cx_reset(&mut self) -> Result<(), Error> {
        Self::init_state(&mut self.state)
    }

    fn cx_header(&mut self) -> &mut cx_hash_t {
        &mut self.state.header
    }

    fn cx_id() -> cx_md_t {
        crate::raw::cx_md_e_CX_BLAKE2B
    }
}
