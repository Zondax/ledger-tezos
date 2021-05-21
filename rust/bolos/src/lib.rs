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

#[macro_use]
extern crate cfg_if;

#[cfg(test)]
extern crate self as bolos_sys;

extern crate no_std_compat as std;

pub use bolos_derive::*;

#[macro_use]
pub mod swapping_buffer;
pub use swapping_buffer::SwappingBuffer;

cfg_if! {
    if #[cfg(feature = "wear")] {
        #[macro_use]
        pub mod wear_leveller;
        pub use wear_leveller::Wear;
    }
}

mod pic;
pub use pic::PIC;

mod nvm;
pub use nvm::NVM;

pub(self) mod raw {
    #![allow(non_snake_case)]
    #![allow(non_upper_case_globals)]
    #![allow(non_camel_case_types)]
    #![allow(dead_code)]
    #![allow(clippy::upper_case_acronyms)]

    cfg_if! {
        if #[cfg(nanos)] {
            include!("./bindings/bindingsS.rs");
        } else if #[cfg(nanox)] {
            include!("./bindings/bindingsX.rs");
        }
    }
}

/// Wrapper for 'os_sched_exit'
/// Exit application with status
pub fn exit_app(status: u8) -> ! {
    cfg_if! {
        if #[cfg(bolos_sdk)] {
            unsafe { raw::os_sched_exit(status as _) }
            unreachable!("Did not exit properly");
        } else {
            panic!("exiting app: {}", status);
        }
    }
}

#[doc(hidden)]
//Please don't use stuff inside here directly
// (there's better wrappers)
pub mod errors;
pub use errors::Error;

pub mod crypto;
