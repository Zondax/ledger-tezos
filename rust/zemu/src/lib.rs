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
#![no_std]
#![no_builtins]
#![allow(dead_code)]

/// cbindgen:ignore
pub(self) mod bindings {
    extern "C" {
        cfg_if::cfg_if! {
            if #[cfg(zemu_sdk)] {
                pub fn zemu_log(buffer: *const u8);
                pub fn check_canary();
                pub fn zemu_log_stack(ctx: *const u8);
            }
        }
    }
}

pub fn zemu_log(_s: &str) {
    #[cfg(zemu_sdk)]
    unsafe {
        let p = _s.as_bytes().as_ptr();
        bindings::zemu_log(p)
    }
}

pub fn zemu_log_stack(_s: &str) {
    #[cfg(zemu_sdk)]
    unsafe {
        let p = _s.as_bytes().as_ptr();
        bindings::zemu_log_stack(p)
    }
}
pub fn check_canary() {
    #[cfg(zemu_sdk)]
    unsafe {
        bindings::check_canary();
    }
}

#[cfg_attr(not(zemu_sdk), path = "ui_mock.rs")]
mod ui;
pub use ui::*;

mod ui_toolkit;
