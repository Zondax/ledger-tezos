use std::ops::Deref;

pub struct NVM<const N: usize>([u8; N]);

impl<const N: usize> NVM<N> {
    pub const fn new() -> Self {
        Self([0; N])
    }

    pub fn write(&mut self, from: usize, slice: &[u8]) -> Result<(), ()> {
        let len = slice.len();
        //if the write wouldn't fit
        // then return error
        if from + len > N {
            return Err(());
        }

        cfg_if::cfg_if! {
            if #[cfg(not(test))] {
                //safety: we got the only possible mutable pointer to this location since
                // we own the location
                unsafe {
                    super::bindings::nvm_write(self.0[from..].as_mut_ptr(), slice.as_ptr(), len as u32);

                    debug_assert_eq!(&self.0[from..], &slice[..]);
                }
            } else {
                self.0[from..from+len].copy_from_slice(slice)
            }
        }

        Ok(())
    }
}

impl<const N: usize> Deref for NVM<N> {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.0[..]
    }
}
