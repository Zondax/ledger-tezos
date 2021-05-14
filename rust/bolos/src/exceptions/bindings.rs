//! Partially taken from ledger sdk
//!
//! https://github.com/LedgerHQ/ledger-nanos-sdk/blob/master/src/bindings.rs

#![allow(non_snake_case)]
#![allow(non_camel_case_types)]

pub type exception_t = u16;
pub type try_context_t = try_context_s;
pub type jmp_buf = [u32; 10usize];

#[repr(C)]
#[derive(Copy, Clone)]
pub struct try_context_s {
    pub jmp_buf: jmp_buf,
    pub previous: *mut try_context_t,
    pub ex: exception_t,
}

impl Default for try_context_s {
    fn default() -> Self {
        unsafe { ::core::mem::zeroed() }
    }
}

extern "C" {
    pub fn setjmp(__jmpb: *mut u32) -> i32;
    pub fn os_longjmp(exception: u32);
    pub fn try_context_set(context: *mut try_context_t) -> *mut try_context_t;
    pub fn try_context_get() -> *mut try_context_t;
}
