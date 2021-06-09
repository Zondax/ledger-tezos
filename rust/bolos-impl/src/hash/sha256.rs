#![allow(unused_imports)]

use crate::raw::{cx_hash_t, cx_sha256_t, cx_md_t};
use crate::{errors::catch, Error};

use super::CxHash;

pub struct Sha256 {
    state: cx_sha256_t,
}

impl Sha256 {
    pub fn new() -> Result<Self, Error> {
        let mut state = Default::default();

        Self::init_state(&mut state)?;

        Ok(Self { state })
    }

    fn init_state(state: &mut cx_sha256_t) -> Result<(), Error> {
        cfg_if! {
            if #[cfg(nanox)] {
                let might_throw = || unsafe {
                    crate::raw::cx_sha256_init(state as *mut _);
                };

                catch(might_throw)?;
            } else if #[cfg(nanos)] {
                match unsafe { crate::raw::cx_sha256_init_no_throw(
                    state as *mut _
                )} {
                    0 => {},
                    err => return Err(err.into()),
                }
            } else {
                todo!("sha256 init called in non-bolos")
            }
        }

        Ok(())
    }
}

impl CxHash<32> for Sha256 {
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
        crate::raw::cx_md_e_CX_SHA256
    }
}
