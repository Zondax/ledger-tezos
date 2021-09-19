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

cfg_if::cfg_if! {
    if #[cfg(not(bolos_sdk))] {

extern crate std;

/// Wrapper for 'os_sched_exit'
/// Exit application with status
pub fn exit_app(status: u8) -> ! {
    panic!("exiting app: {}", status);
}

pub mod pic;
pub use pic::PIC;

pub mod nvm;
pub use nvm::NVM;

#[doc(hidden)]
pub mod errors;
pub use errors::Error;

pub mod crypto;
pub mod hash;
pub mod hmac;

mod panic {
    #[macro_export]
    macro_rules! panic_handler {
        ($($body:tt)*) => {};
    }
}

    }
}
