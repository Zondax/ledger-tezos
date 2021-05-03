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
#![macro_use]

extern crate no_std_compat as std;
use std::prelude::v1::*;

cfg_if::cfg_if! {
    if #[cfg(not(test))] {
        use core::panic::PanicInfo;

        #[panic_handler]
        fn panic(_info: &PanicInfo) -> ! {
            loop {}
        } // TODO: we should reset the device here
    }
}

pub mod constants;
pub mod dispatcher;
mod bolos;
mod handlers;
mod utils;

use bolos::{check_canary, zemu_log};
use dispatcher::handle_apdu;

/// # Safety
///
/// This function is the app entry point for the minimal C stub
#[no_mangle]
pub unsafe extern "C" fn rs_handle_apdu(
    _flags: *mut u32,
    _tx: *mut u32,
    rx: u32,
    buffer: *mut u8,
    buffer_len: u16,
) {
    let flags = _flags.as_mut().unwrap();
    let tx = _tx.as_mut().unwrap();
    let data = std::slice::from_raw_parts_mut(buffer, buffer_len as usize);
    zemu_log("rs_handle_apdu\n\x00");

    handle_apdu(flags, tx, rx, data);

    check_canary();
}
