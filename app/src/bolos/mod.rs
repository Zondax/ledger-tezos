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
// FIXME: Refactor so zemu and bolos-FFI are clearly separated as xxx-sys crates
#![allow(dead_code)]

mod buffer;
pub use buffer::*;

mod nvm;

extern "C" {
    #[cfg(not(test))]
    #[link_name = "zemu_log"]
    pub fn c_zemu_log(buffer: *const u8);

    #[cfg(not(test))]
    #[link_name = "check_canary"]
    fn c_check_canary();

    #[cfg(not(test))]
    fn pic(link_address: u32) -> u32;

    #[cfg(not(test))]
    #[link_name = "nvm_write"]
    pub fn c_nvm_write(dest: *mut u8, src: *const u8, len: u32);
}

pub fn zemu_log(_s: &str) {
    #[cfg(not(test))]
    unsafe {
        let p = _s.as_bytes().as_ptr();
        c_zemu_log(p)
    }
}

pub(crate) fn check_canary() {
    #[cfg(not(test))]
    unsafe {
        c_check_canary();
    }
}

//https://github.com/LedgerHQ/ledger-nanos-sdk/blob/master/src/lib.rs#L179
/// This struct is to be used when dealing with code memory spaces
/// as the memory is mapped differently once the app is installed.
///
/// This struct should then be used when accessing `static` memory or
/// function pointers (const in rust is optimized at compile-time)
///
/// # Example
/// ```
/// //BUFFER is a `static` so we need to wrap it with PIC so it would
/// //be accessible when running under BOLOS
/// static BUFFER: Pic<[u8; 1024]> = PIC::new([0; 1024])
///
/// assert_eq!(&[0; 1024], BUFFER);
/// ```
pub struct PIC<T> {
    data: T,
}

impl<T> PIC<T> {
    pub const fn new(data: T) -> Self {
        Self { data }
    }

    pub fn get_ref(&self) -> &T {
        cfg_if::cfg_if! {
            if #[cfg(not(test))] {
                let ptr = unsafe { pic(&self.data as *const T as u32) as *const T };
                unsafe { &*ptr }
            } else {
                &self.data
            }
        }
    }

    /// Warning: this should be used only in conjunction with `nvm_write`
    pub fn get_mut(&mut self) -> &mut T {
        cfg_if::cfg_if! {
            if #[cfg(not(test))] {
                let ptr = unsafe { pic(&mut self.data as *mut T as u32) as *mut T };
                unsafe { &mut *ptr }
            } else {
                &mut self.data
            }
        }
    }
}

impl<T> core::ops::Deref for PIC<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.get_ref()
    }
}

impl<T> core::ops::DerefMut for PIC<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.get_mut()
    }
}

impl<T: Default> Default for PIC<T> {
    fn default() -> Self {
        PIC::new(T::default())
    }
}
