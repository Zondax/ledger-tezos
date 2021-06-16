#![allow(unused_imports)]

use crate::raw::{cx_hash_t, cx_md_t, cx_sha512_t};
use crate::{errors::catch, Error};

use super::CxHash;

pub struct Sha512 {
    state: cx_sha512_t,
}

impl Sha512 {
    pub fn new() -> Result<Self, Error> {
        let mut this = Self {
            state: Default::default(),
        };

        Self::init_state(&mut this.state)?;

        Ok(this)
    }

    fn init_state(state: &mut cx_sha512_t) -> Result<(), Error> {
        cfg_if! {
            if #[cfg(nanox)] {
                let might_throw = || unsafe {
                    crate::raw::cx_sha512_init(state as *mut _);
                };

                catch(might_throw)?;
            } else if #[cfg(nanos)] {
                match unsafe { crate::raw::cx_sha512_init_no_throw(
                    state as *mut _
                )} {
                    0 => {},
                    err => return Err(err.into()),
                }
            } else {
                todo!("sha512init called in non-bolos")
            }
        }

        Ok(())
    }
}

impl CxHash<64> for Sha512 {
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
        crate::raw::cx_md_e_CX_SHA512
    }
}