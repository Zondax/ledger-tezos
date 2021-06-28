use std::ops::Deref;

use crate::Error as SysError;

/// This struct is to be used when wanting to store something in non-volatile
/// memory (NVM).
///
/// Often used in conjunction with [PIC].
///
/// # Example
/// ```
/// # use bolos::{PIC, NVM};
/// //the macro will take care of wrapping with PIC aswell
/// #[bolos::nvm]
/// static MEMORY: [u8; 1024];
///
/// let _: &PIC<NVM<1024>> = &MEMORY;
/// assert_eq!(&[0; 1024], &**MEMORY);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct NVM<const N: usize>([u8; N]);

#[derive(Debug, Clone, Copy)]
pub enum NVMError {
    Overflow { max: usize, got: usize },
    Internal(SysError),
}

impl From<SysError> for NVMError {
    fn from(err: SysError) -> Self {
        Self::Internal(err)
    }
}

impl<const N: usize> NVM<N> {
    pub const fn zeroed() -> Self {
        Self([0; N])
    }

    pub const fn new(data: [u8; N]) -> Self {
        Self(data)
    }

    pub fn write(&mut self, from: usize, slice: &[u8]) -> Result<(), NVMError> {
        let len = slice.len();
        //if the write wouldn't fit
        // then return error
        if from + len > N {
            return Err(NVMError::Overflow {
                max: N,
                got: from + len,
            });
        }

        self.0[from..from + len].copy_from_slice(slice);

        Ok(())
    }

    /// This function is unsafe because you shouldn't be writing to this slice directly
    ///
    /// # Safety
    /// To correctly write to the underlying slice, it's important that this struct or `nvm_write`
    /// is used, otherwise the write will fail
    pub unsafe fn get_mut(&mut self) -> &mut [u8; N] {
        &mut self.0
    }

    pub fn read(&self) -> &[u8; N] {
        &self.0
    }
}

impl<const N: usize> Deref for NVM<N> {
    type Target = [u8; N];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
