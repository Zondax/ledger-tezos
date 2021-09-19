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

//! This crate provides implementations and wrappers around `bolos-sys`

#![no_std]
#![no_builtins]
#![allow(non_upper_case_globals)]

#[macro_use]
extern crate cfg_if;

extern crate no_std_compat as std;

cfg_if! {
    if #[cfg(bolos_sdk)] {
//----------------------------

pub(self) use bolos_sys::raw;

pub use bolos_sys::exit_app;

pub use bolos_sys::pic;
pub use bolos_sys::pic::PIC;

pub mod nvm;
pub use nvm::NVM;

#[doc(hidden)]
//Please don't use stuff inside here directly
// (there's better wrappers)
pub mod errors;
pub use errors::Error;

pub mod crypto;
pub mod hash;
pub mod hmac;

/// Provides a macro to register a panic handler with this crate
mod panic {
    #[macro_export]
    macro_rules! panic_handler {
        ($($body:tt)*) => {
            mod __panic_handler {
                use core::panic::PanicInfo;

                #[panic_handler]
                fn panic(_info: &PanicInfo) -> ! {
                    $($body)*;
                    $crate::exit_app(255);
                }
            }
        };
    }
}

/// Provides miscellaneous utilities for types in this crate
mod misc;
//------------------------
    }
}
