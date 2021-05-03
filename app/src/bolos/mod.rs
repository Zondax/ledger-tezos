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

mod pic;
pub use pic::PIC;

mod nvm;

pub(self) mod bindings {
    extern "C" {
        cfg_if::cfg_if! {
            if #[cfg(not(test))] {
                pub fn zemu_log(buffer: *const u8);
                pub fn check_canary();
                pub fn pic(link_address: u32) -> u32;
                pub fn nvm_write(dest: *mut u8, src: *const u8, len: u32);
            }
        }
    }
}

pub fn zemu_log(_s: &str) {
    #[cfg(not(test))]
    unsafe {
        let p = _s.as_bytes().as_ptr();
        bindings::zemu_log(p)
    }
}

pub(crate) fn check_canary() {
    #[cfg(not(test))]
    unsafe {
        bindings::check_canary();
    }
}
