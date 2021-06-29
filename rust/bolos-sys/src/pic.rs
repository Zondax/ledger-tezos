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
use core::ops::{Deref, DerefMut};

//https://github.com/LedgerHQ/ledger-nanos-sdk/blob/master/src/lib.rs#L179
/// This struct is to be used when dealing with code memory spaces
/// as the memory is mapped differently once the app is installed.
///
/// This struct should then be used when accessing flash memory (via nvm or immutable statics) or
/// function pointers (const in rust is optimized at compile-time)
///
/// # Example
/// ```
/// # use bolos::PIC;
/// //BUFFER is a `static` so we need to wrap it with PIC so it would
/// //be accessible when running under BOLOS
/// #[bolos::pic]
/// static BUFFER: [u8; 1024] = [0; 1024];
///
/// let _: &PIC<[u8; 1024]> = &BUFFER;
/// assert_eq!(&[0; 1024], &*BUFFER);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct PIC<T> {
    data: T,
}

impl<T> PIC<T> {
    pub const fn new(data: T) -> Self {
        Self { data }
    }

    pub fn get_ref(&self) -> &T {
        cfg_if::cfg_if! {
            if #[cfg(bolos_sdk)] {
                let ptr = unsafe { super::raw::pic(&self.data as *const T as _) as *const T };
                unsafe { &*ptr }
            } else {
                &self.data
            }
        }
    }

    /// Warning: this should be used only in conjunction with `nvm_write`
    pub fn get_mut(&mut self) -> &mut T {
        cfg_if::cfg_if! {
            if #[cfg(bolos_sdk)] {
                let ptr = unsafe { super::raw::pic(&mut self.data as *mut T as _) as *mut T };

                unsafe { &mut *ptr }
            } else {
                &mut self.data
            }
        }
    }

    pub fn into_inner(self) -> T {
        cfg_if::cfg_if! {
            if #[cfg(bolos_sdk)] {
                //no difference afaik from &mut and & in this case, since we consume self
                let ptr = unsafe { super::raw::pic(&self.data as *const T as _) as *const T };

                //we don't want to drop the old location
                //if the location is unchanged then it will be dropped later anyways
                core::mem::forget(self);

                unsafe { ptr.read() }
            } else {
                self.data
            }
        }
    }
}

impl PIC<()> {
    //Apply pic manually, interpreting `ptr` as the actual pointer to an _unknwon_ type
    pub unsafe fn manual(ptr: usize) -> usize {
        cfg_if::cfg_if! {
            if #[cfg(bolos_sdk)] {
                let ptr = super::raw::pic(ptr as _) as usize;

                ptr
            } else {
                ptr
            }
        }
    }
}

impl<T> Deref for PIC<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.get_ref()
    }
}

impl<T> DerefMut for PIC<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.get_mut()
    }
}

impl<T: Default> Default for PIC<T> {
    fn default() -> Self {
        PIC::new(T::default())
    }
}
