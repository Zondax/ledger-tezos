use std::ops::{Deref, DerefMut};

//https://github.com/LedgerHQ/ledger-nanos-sdk/blob/master/src/lib.rs#L179
/// This struct is to be used when dealing with code memory spaces
/// as the memory is mapped differently once the app is installed.
///
/// This struct should then be used when accessing `static` memory or
/// function pointers (const in rust is optimized at compile-time)
///
/// # Example
/// ```
/// //BUFFER is a `static` so we need to wrap it with PIC so it would
/// //be accessible when running under BOLOS
/// static BUFFER: Pic<[u8; 1024]> = PIC::new([0; 1024])
///
/// assert_eq!(&[0; 1024], BUFFER);
/// ```
pub struct PIC<T> {
    data: T,
}

impl<T> PIC<T> {
    pub const fn new(data: T) -> Self {
        Self { data }
    }

    pub fn get_ref(&self) -> &T {
        cfg_if::cfg_if! {
            if #[cfg(bolos_sdk)] {
                let ptr = unsafe { super::bindings::pic(&self.data as *const T as u32) as *const T };
                unsafe { &*ptr }
            } else {
                &self.data
            }
        }
    }

    /// Warning: this should be used only in conjunction with `nvm_write`
    pub fn get_mut(&mut self) -> &mut T {
        cfg_if::cfg_if! {
            if #[cfg(bolos_sdk)] {
                let ptr = unsafe { super::bindings::pic(&mut self.data as *mut T as u32) as *mut T };
                unsafe { &mut *ptr }
            } else {
                &mut self.data
            }
        }
    }
}

impl<T> Deref for PIC<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.get_ref()
    }
}

impl<T> DerefMut for PIC<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.get_mut()
    }
}

impl<T: Default> Default for PIC<T> {
    fn default() -> Self {
        PIC::new(T::default())
    }
}
