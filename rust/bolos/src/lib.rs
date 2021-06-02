#![no_std]
#![no_builtins]

extern crate no_std_compat as std;

#[macro_use]
extern crate cfg_if;

pub use bolos_derive::*;

cfg_if! {
    if #[cfg(feature = "wear")] {
        #[macro_use]
        pub mod wear_leveller;
        pub use wear_leveller::Wear;
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
