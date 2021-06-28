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

mod panic {
    #[macro_export]
    macro_rules! panic_handler {
        ($($body:tt)*) => {};
    }
}

    }
}
