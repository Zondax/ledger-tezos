#![allow(unused_imports)]

use crate::raw::{cx_blake2b_t, cx_hash_t};
use crate::{errors::catch, Error};

use super::CxHash;

pub struct Blake2b<const S: usize> {
    state: cx_blake2b_t,
}

impl<const S: usize> Blake2b<S> {
    pub fn new() -> Result<Self, Error> {
        Self::init_hasher()
    }
}

impl<const S: usize> CxHash<S> for Blake2b<S> {
    fn init_hasher() -> Result<Self, Error> {
        let mut state = cx_blake2b_t::default();

        cfg_if! {
            if #[cfg(nanox)] {
                let might_throw = || unsafe {
                    crate::raw::cx_blake2b_init(&mut state as *mut _, (S * 8) as u32);
                };

                catch(might_throw)?;
            } else if #[cfg(nanos)] {
                match unsafe { crate::raw::cx_blake2b_init_no_throw(
                    &mut state as *mut _,
                    (S * 8) as u32
                )} {
                    0 => {},
                    err => return Err(err.into()),
                }
            } else {
                todo!("blake2b init called in non bolos")
            }
        }

        Ok(Self { state })
    }

    fn cx_header(&mut self) -> &mut cx_hash_t {
        &mut self.state.header
    }
}
