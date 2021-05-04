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

//! This crate provides bindings for Ledger's BOLOS, as well as wrappers and utilities

extern crate no_std_compat as std;
use std::prelude::v1::*;

#[macro_use]
pub mod swapping_buffer;

mod pic;
pub use pic::PIC;

mod nvm;
pub use nvm::NVM;

pub(self) mod bindings {
    extern "C" {
        cfg_if::cfg_if! {
            if #[cfg(bolos_sdk)] {
                pub fn pic(link_address: u32) -> u32;
                pub fn nvm_write(dest: *mut u8, src: *const u8, len: u32);
            }
        }
    }
}
