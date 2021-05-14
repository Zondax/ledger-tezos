#![no_std]
#![no_builtins]
#![allow(dead_code)]

pub(self) mod bindings {
    extern "C" {
        cfg_if::cfg_if! {
            if #[cfg(zemu_sdk)] {
                pub fn zemu_log(buffer: *const u8);
                pub fn check_canary();
            }
        }
    }
}

fn zemu_log(_s: &str) {
    #[cfg(zemu_sdk)]
    unsafe {
        let p = _s.as_bytes().as_ptr();
        bindings::zemu_log(p)
    }
}

pub fn check_canary() {
    #[cfg(zemu_sdk)]
    unsafe {
        bindings::check_canary();
    }
}

mod zemu_logger;
pub use zemu_logger::ZemuLog;
