#![allow(unused_imports)]

use crate::raw::{cx_hash_t, cx_sha256_t};
use crate::{errors::catch, Error};

use super::CxHash;

pub struct Sha256 {
    state: cx_sha256_t,
}

impl Sha256 {
    pub fn new() -> Result<Self, Error> {
        Self::init_hasher()
    }
}

impl CxHash<32> for Sha256 {
    fn init_hasher() -> Result<Self, Error> {
        let mut state = cx_sha256_t::default();

        cfg_if! {
            if #[cfg(nanox)] {
                let might_throw = || unsafe {
                    crate::raw::cx_sha256_init(&mut state as *mut _);
                };

                catch(might_throw)?;
            } else if #[cfg(nanos)] {
                match unsafe { crate::raw::cx_sha256_init_no_throw(
                    &mut state as *mut _
                )} {
                    0 => {},
                    err => return Err(err.into()),
                }
            } else {
                todo!("sha256 init called in non-bolos")
            }
        }

        Ok(Self { state })
    }

    fn cx_header(&mut self) -> &mut cx_hash_t {
        &mut self.state.header
    }
}
