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

#[cfg(bolos_sdk)]
pub mod raw {
    #![allow(non_snake_case)]
    #![allow(non_upper_case_globals)]
    #![allow(non_camel_case_types)]
    #![allow(dead_code)]
    #![allow(clippy::upper_case_acronyms)]
    #![allow(non_upper_case_globals)]

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
#[cfg(bolos_sdk)]
pub fn exit_app(status: u8) -> ! {
    unsafe { raw::os_sched_exit(status as _) }
    unsafe { core::hint::unreachable_unchecked() }
}

/// Contains some impls for items coming from the bindings
#[cfg(bolos_sdk)]
mod extra_traits;
