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
use std::ops::Deref;

use crate::{errors::catch, Error as SysError};

/// This struct is to be used when wanting to store something in non-volatile
/// memory (NVM).
///
/// Often used in conjunction with [super::pic::PIC].
///
/// # Example
/// ```
/// # use bolos::{PIC, NVM};
/// //the macro will take care of wrapping with PIC aswell
/// #[bolos::nvm]
/// static MEMORY: [u8; 1024];
///
/// let _: &PIC<NVM<1024>> = &MEMORY;
/// assert_eq!(&[0; 1024], &**MEMORY);
/// ```
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(align(64))]
pub struct NVM<const N: usize>([u8; N]);

#[derive(Clone, Copy)]
pub enum NVMError {
    Overflow { max: usize, got: usize },
    Internal(SysError),
}

impl From<SysError> for NVMError {
    fn from(err: SysError) -> Self {
        Self::Internal(err)
    }
}

impl<const N: usize> NVM<N> {
    pub const fn zeroed() -> Self {
        Self([0; N])
    }

    pub const fn new(data: [u8; N]) -> Self {
        Self(data)
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

                catch(write)?;
            } else {
                self.0[from..from+len].copy_from_slice(slice)
            }
        }

        Ok(())
    }

    /// This function is unsafe because you shouldn't be writing to this slice directly
    pub unsafe fn get_mut(&mut self) -> &mut [u8; N] {
        &mut self.0
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

mod manual {
    #![allow(dead_code)]

    /// This struct is to be used when wanting to properly write the memory behind the pointer
    /// but the memory is not actually owned by the application, or the pointer has been obtained
    /// another way
    pub struct ManualNVM<'m> {
        ptr: *mut u8,
        len: usize,
        _p: std::marker::PhantomData<&'m ()>,
    }

    impl<'m> ManualNVM<'m> {
        pub fn new(p: std::ptr::NonNull<u8>, len: usize) -> Self {
            Self {
                ptr: p.as_ptr(),
                len,
                _p: Default::default(),
            }
        }

        /// This function is unsafe because we can't guarantee that `self.ptr` is  _actually_
        /// a pointer to NVM
        pub unsafe fn write(&mut self, from: usize, slice: &[u8]) -> Result<(), ()> {
            let len = slice.len();
            //if the write wouldn't fit
            // then return error
            if from + len > self.len {
                return Err(());
            }

            cfg_if! {
                if #[cfg(bolos_sdk)] {
                    let p = self.ptr.add(from);
                    crate::raw::nvm_write(p as *mut _, slice.as_ptr() as *mut u8 as *mut _, len as u32);
                } else {
                    let mem: &'m mut [u8] = std::slice::from_raw_parts_mut(self.ptr, self.len);
                    mem[from..from+len].copy_from_slice(slice)
                }
            }

            Ok(())
        }
    }
}
