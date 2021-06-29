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

extern crate no_std_compat as std;

extern crate self as bolos;

#[macro_use]
extern crate cfg_if;

pub use bolos_derive::*;

cfg_if! {
    if #[cfg(feature = "flash-slot")] {
        #[macro_use]
        pub mod flash_slot;
        pub use flash_slot::Wear;
    }
}

#[macro_use]
pub mod swapping_buffer;
pub use swapping_buffer::*;

cfg_if! {
    if #[cfg(all(__impl, __mock))] {
        compiler_error!("Can't have both `__impl` and `__mock` enabled");
    } else if #[cfg(__impl)] {
        pub use bolos_impl::*;
    } else if #[cfg(__mock)] {
        pub use bolos_mock::*;
    } else {
        compile_error!("Need either `__impl` or `__mock` feature enabled");
    }
}
