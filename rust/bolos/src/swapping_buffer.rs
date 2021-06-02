use super::{nvm::{NVMError, NVM}, PIC};
use std::prelude::v1::*;

#[derive(Debug, Clone, Copy)]
enum BufferState {
    WritingToRam(usize),
    WritingToFlash(usize),
}

impl Default for BufferState {
    fn default() -> Self {
        BufferState::WritingToRam(0)
    }
}

impl BufferState {
    ///Pass to the next state, RAM -> FLASH
    pub fn transition_forward(&mut self) -> Result<(), ()> {
        match self {
            BufferState::WritingToRam(cnt) => {
                *self = Self::WritingToFlash(*cnt);
                Ok(())
            }
            _ => Err(()),
        }
    }
}

#[cfg(test)]
impl BufferState {
    const fn is_ram(&self) -> bool {
        match self {
            Self::WritingToRam(_) => true,
            _ => false,
        }
    }

    const fn is_flash(&self) -> bool {
        !self.is_ram()
    }
}

/// This struct is used to manage 2 buffers, with one "working" buffer
/// and a "fallback" buffer when the first one is too small for the attempted operation
pub struct SwappingBuffer<'r, 'f, const RAM: usize, const FLASH: usize> {
    ram: &'r mut [u8; RAM],
    flash: &'f mut PIC<NVM<FLASH>>,
    state: BufferState,
}

impl<'r, 'f, const RAM: usize, const FLASH: usize> SwappingBuffer<'r, 'f, RAM, FLASH> {
    /// Create a new instance of the buffer
    pub fn new(ram: &'r mut [u8; RAM], flash: &'f mut PIC<NVM<FLASH>>) -> Self {
        Self {
            ram,
            flash,
            state: Default::default(),
        }
    }

    /// Will return the entire underlying buffer as an immutable slice
    pub fn read(&self) -> &[u8] {
        match self.state {
            BufferState::WritingToRam(_) => &self.ram[..],
            BufferState::WritingToFlash(_) => &self.flash[..],
        }
    }

    /// Will return the underlying written buffer as an immutable slice
    pub fn read_exact(&self) -> &[u8] {
        match self.state {
            BufferState::WritingToRam(cnt) => &self.ram[..cnt],
            BufferState::WritingToFlash(cnt) => &self.flash[..cnt],
        }
    }

    /// Will attempt to append to the underlying buffer,
    /// switching to the second buffer if needed.
    ///
    /// Will copy the first buffer into the second before switching.
    /// Switching is permanent unless [`reset`] is called
    ///
    /// # Errors
    /// This function will error if the second buffer is smaller than the requested amount,
    /// either when appending or when moving from the first buffer,
    /// or if there's an exception when writing to NVM
    pub fn write(&mut self, bytes: &[u8]) -> Result<(), NVMError> {
        let len = bytes.len();

        match &mut self.state {
            //if we writing to ram but there's not enough space for this coming write
            BufferState::WritingToRam(cnt) if *cnt + len > RAM => {
                //copy ram to flash, and move state over to flash
                self.flash.write(0, &*self.ram)?;
                self.state.transition_forward().unwrap();

                //then write (counter already incremented)
                self.write(bytes)?;
                Ok(())
            }
            //writing to ram and we have space
            BufferState::WritingToRam(cnt) => {
                //copy slice and update counter
                self.ram[*cnt..*cnt + len].copy_from_slice(bytes);
                *cnt += len;
                Ok(())
            }
            //writing to flash and no more space, error
            BufferState::WritingToFlash(cnt) if *cnt + len > FLASH => Err(NVMError::Overflow {
                max: FLASH,
                got: *cnt + len,
            }),
            //writing to flash try to write and update counter in case of success
            BufferState::WritingToFlash(cnt) => {
                //this will never throw a size error, just a write exception
                self.flash.write(*cnt, bytes)?;
                *cnt += len;
                Ok(())
            }
        }
    }

    /// Reset the buffer counter and state to the initial configuration
    ///
    /// # Warning
    /// Will not overwrite the buffer contents
    pub fn reset(&mut self) {
        self.state = Default::default();
    }
}

#[cfg(test)]
#[allow(dead_code)]
impl<'r, 'f, const RAM: usize, const FLASH: usize> SwappingBuffer<'r, 'f, RAM, FLASH> {
    const fn sizes(&self) -> (usize, usize) {
        (RAM, FLASH)
    }

    fn state(&self) -> BufferState {
        self.state
    }
}

#[macro_export]
macro_rules! new_swapping_buffer {
    ($ram:expr, $flash:expr) => {{
        static mut __RAM: [u8; $ram] = [0; $ram];

        #[$crate::nvm]
        static mut __FLASH: [u8; $flash];

        unsafe { $crate::SwappingBuffer::new(&mut __RAM, &mut __FLASH) }
    }};
}

#[cfg(test)]
mod tests {
    use super::*;
    const MSG: &[u8] = b"deadbeef";

    #[test]
    fn macro_works() {
        let buffer = new_swapping_buffer!(1, 2);

        assert_eq!((1, 2), buffer.sizes());
        assert!(buffer.state.is_ram());
    }

    #[test]
    fn no_ram() {
        let mut buffer = new_swapping_buffer!(0, 8);

        //should be able to write
        buffer.write(MSG).unwrap();
        assert!(buffer.state.is_flash()); //and be in flash

        //should be readable
        assert_eq!(MSG, buffer.read());

        //full
        assert!(buffer.write(MSG).is_err())
    }

    #[test]
    fn no_flash() {
        let mut buffer = new_swapping_buffer!(8, 0);

        buffer.write(MSG).unwrap();
        assert!(buffer.state.is_ram()); //should all be in ram

        //should be readable
        assert_eq!(MSG, buffer.read());

        //full
        assert!(buffer.write(MSG).is_err());
    }

    #[test]
    fn incremental_ram() {
        let mut buffer = new_swapping_buffer!(16, 0);

        buffer.write(MSG).unwrap();
        assert!(buffer.state.is_ram());

        buffer.write(MSG).unwrap();
        assert!(buffer.state.is_ram());

        assert!(buffer.write(MSG).is_err());

        //find first occurence of MSG in buffer
        assert!(buffer
            .read()
            .windows(MSG.len())
            .position(|w| w == MSG)
            .is_some())
    }

    #[test]
    fn incremental_flash() {
        let mut buffer = new_swapping_buffer!(0, 16);

        buffer.write(MSG).unwrap();
        assert!(buffer.state.is_flash());

        buffer.write(MSG).unwrap();
        assert!(buffer.state.is_flash());

        assert!(buffer.write(MSG).is_err());

        //should find only MSG in the buffer
        assert!(buffer.read().chunks(MSG.len()).all(|c| c == MSG))
    }

    #[test]
    fn transition() {
        let mut buffer = new_swapping_buffer!(4, 8);

        //write 8 bytes
        buffer.write(MSG).unwrap();
        assert!(buffer.state.is_flash()); //should move to second buffer

        assert!(buffer.write(MSG).is_err());

        assert_eq!(MSG, buffer.read());
    }

    #[test]
    #[should_panic]
    fn not_enough_space() {
        let mut buffer = new_swapping_buffer!(4, 7);

        //writing 8 bytes will try to write to second buffer
        // but will fail since no space there either
        buffer.write(MSG).unwrap();
    }

    #[test]
    fn reset() {
        let mut buffer = new_swapping_buffer!(8, 16);

        buffer.write(MSG).unwrap();
        assert!(buffer.state.is_ram());

        buffer.write(MSG).unwrap();
        assert!(buffer.state.is_flash());

        //check if all chunks are MSG
        assert!(buffer.read().chunks(MSG.len()).all(|c| c == MSG));

        buffer.reset();

        assert!(buffer.state.is_ram());
        assert!(buffer.read_exact().is_empty());
    }
}
