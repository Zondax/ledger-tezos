use std::ops::{Deref, DerefMut};

//https://github.com/LedgerHQ/ledger-nanos-sdk/blob/master/src/lib.rs#L179
/// This struct is to be used when dealing with code memory spaces
/// as the memory is mapped differently once the app is installed.
///
/// This struct should then be used when accessing flash memory (via nvm or immutable statics) or
/// function pointers (const in rust is optimized at compile-time)
///
/// # Example
/// ```
/// # use bolos::PIC;
/// //BUFFER is a `static` so we need to wrap it with PIC so it would
/// //be accessible when running under BOLOS
/// #[bolos::pic]
/// static BUFFER: [u8; 1024] = [0; 1024];
///
/// let _: &PIC<[u8; 1024]> = &BUFFER;
/// assert_eq!(&[0; 1024], &*BUFFER);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct PIC<T> {
    data: T,
}

impl<T> PIC<T> {
    pub const fn new(data: T) -> Self {
        Self { data }
    }

    pub fn get_ref(&self) -> &T {
        &self.data
    }

    /// Warning: this should be used only in conjunction with `nvm_write`
    pub fn get_mut(&mut self) -> &mut T {
        &mut self.data
    }

    pub fn into_inner(self) -> T {
        self.data
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
