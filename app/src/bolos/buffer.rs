use super::{nvm::NVM, PIC};

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

/// This struct is used to manage 2 buffers, with one "working" buffer
/// and a "fallback" buffer when the first one is too small for the attempted operation
pub struct SwappingBuffer<'r, 'f, const RAM: usize, const FLASH: usize> {
    ram: &'r mut PIC<[u8; RAM]>,
    flash: &'f mut PIC<NVM<FLASH>>,
    state: BufferState,
}

impl<'r, 'f, const RAM: usize, const FLASH: usize> SwappingBuffer<'r, 'f, RAM, FLASH> {
    /// Create a new instance of the buffer
    pub fn new(ram: &'r mut PIC<[u8; RAM]>, flash: &'f mut PIC<NVM<FLASH>>) -> Self {
        Self {
            ram,
            flash,
            state: Default::default(),
        }
    }

    pub fn read(&self) -> &[u8] {
        match self.state {
            BufferState::WritingToRam(_) => &self.ram[..],
            BufferState::WritingToFlash(_) => self.flash,
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
    /// either when appending or when moving from the first buffer
    pub fn write(&mut self, bytes: &[u8]) -> Result<(), ()> {
        let len = bytes.len();

        match &mut self.state {
            //if we writing to ram but there's not enough space for this coming write
            BufferState::WritingToRam(cnt) if *cnt + len > RAM => {
                //copy ram to flash, and move state over to flash
                self.flash.write(0, &**self.ram)?;
                self.state.transition_forward().unwrap();

                //then write (counter already incremented)
                self.write(bytes)?;
                Ok(())
            }
            //writing to ram and we have space
            BufferState::WritingToRam(cnt) => {
                //copy slice and update counter
                self.ram[*cnt..len].copy_from_slice(bytes);
                *cnt += len;
                Ok(())
            }
            //writing to flash and no more space, error
            BufferState::WritingToFlash(cnt) if *cnt + len > FLASH => Err(()),
            //writing to flash try to write and update counter in case of success
            BufferState::WritingToFlash(cnt) => {
                //this is ok because we check already for the size
                self.flash.write(*cnt, bytes).unwrap();
                *cnt += len;
                Ok(())
            }
        }
    }

    /// Reset the buffer counter and state to the initial configuration
    pub fn reset(&mut self) {
        self.state = Default::default();
    }
}

#[macro_export]
macro_rules! new {
    ($ram:expr, $flash:expr) => {
        use super::*;
        static mut RAM: PIC<[u8; $ram]> = PIC::new([0; $ram]);

        #[link_section = ".nvram_data"]
        static mut FLASH: PIC<NVM<$flash>> = PIC::new(NVM::new());

        unsafe { SwappingBuffer::new(&mut RAM, &mut FLASH) }
    };
}
