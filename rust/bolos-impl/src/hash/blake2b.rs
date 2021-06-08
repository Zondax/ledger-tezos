#![allow(unused_imports)]

use crate::raw::{cx_blake2b_t, cx_hash_t};
use crate::{errors::catch, Error};

use super::CxHash;

pub struct Blake2b<const S: usize> {
    state: cx_blake2b_t,
}

impl<const S: usize> Blake2b<S> {
    pub fn new() -> Result<Self, Error> {
        let mut state = Default::default();

        Self::init_state(&mut state)?;

        Ok(Self { state })
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
    fn init_hasher() -> Result<Self, Error> {
        Self::new()
    }

    fn reset(&mut self) -> Result<(), Error> {
        Self::init_state(&mut self.state)
    }

    fn cx_header(&mut self) -> &mut cx_hash_t {
        &mut self.state.header
    }
}
