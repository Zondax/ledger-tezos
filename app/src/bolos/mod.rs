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

extern "C" {
    #[cfg(not(test))]
    #[link_name = "zemu_log"]
    pub fn c_zemu_log(buffer: *const u8);

    #[cfg(not(test))]
    #[link_name = "check_canary"]
    fn c_check_canary();

    #[cfg(not(test))]
    fn pic(link_address: u32) -> u32;
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

#[cfg(not(test))]
pub fn pic_internal<T: Sized>(obj: &T) -> &T {
    let ptr = obj as *const _;
    let ptr_usize = ptr as *const () as u32;
    unsafe {
        let link = pic(ptr_usize);
        let ptr = link as *const T;
        &*ptr
    }
}

#[macro_export]
macro_rules! pic {
    ($obj:expr) => {{
        #[cfg(not(test))]
        {
            use crate::pic_internal;
            pic_internal(&$obj)
        }
        return $obj;
    }};
}
