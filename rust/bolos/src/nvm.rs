use std::ops::Deref;

use crate::{exceptions::catch_exception, SyscallError};

pub struct NVM<const N: usize>([u8; N]);

#[derive(Debug, Clone, Copy)]
pub enum NVMError {
    Overflow { max: usize, got: usize },
    Internal(SyscallError),
}

impl From<SyscallError> for NVMError {
    fn from(err: SyscallError) -> Self {
        Self::Internal(err)
    }
}

impl<const N: usize> NVM<N> {
    pub const fn new() -> Self {
        Self([0; N])
    }

    pub fn write(&mut self, from: usize, slice: &[u8]) -> Result<(), NVMError> {
        let len = slice.len();
        //if the write wouldn't fit
        // then return error
        if from + len > N {
            return Err(NVMError::Overflow {
                max: N,
                got: from + len,
            });
        }

        cfg_if! {
            if #[cfg(bolos_sdk)] {
                //safety: we got the only possible mutable pointer to this location since
                // we own the location
                let write = || unsafe {
                    let dst = self.0[from..].as_mut_ptr() as *mut _;
                    let src = slice.as_ptr() as *mut u8 as *mut _;
                    super::raw::nvm_write(dst, src, len as u32);

                    debug_assert_eq!(&self.0[from..], &slice[..]);
                };

                catch_exception::<NVMError, _, _>(write)?;
            } else {
                self.0[from..from+len].copy_from_slice(slice)
            }
        }

        Ok(())
    }

    pub fn read(&self) -> &[u8; N] {
        &self.0
    }
}

impl<const N: usize> Deref for NVM<N> {
    type Target = [u8; N];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
