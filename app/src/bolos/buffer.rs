use super::{PIC, nvm::NVM};

/// This struct is used to manage 2 buffers, with one "working" buffer
/// and a "fallback" buffer when the first one is too small for the attempted operation
pub struct SwappingBuffer<'r, 'f, const RAM: usize, const FLASH: usize> {
    ram: &'r mut PIC<[u8; RAM]>,
    flash: &'f mut PIC<NVM<FLASH>>,
    second: bool,
}

impl<'r, 'f, const RAM: usize, const FLASH: usize> SwappingBuffer<'r, 'f, RAM, FLASH> {
    /// Create a new instance of the buffer
    pub fn new(ram: &'r mut PIC<[u8; RAM]>, flash: &'f mut PIC<NVM<FLASH>>) -> Self {
        Self { ram, flash, second: false }
    }

    pub fn read(&self) -> &[u8] {
        match self.second {
            true => {
                self.flash
            },
            false => {
                &self.ram[..]
            }
        }
    }

    pub fn write(&mut self, bytes: &[u8]) -> Result<(), ()>{
        let len = bytes.len();
        if len > RAM {
            if len > FLASH {
                return Err(())
            }

            self.second = true;
            self.flash.write(bytes)
        } else {
            self.ram[..len].copy_from_slice(bytes);

            Ok(())
        }
    }
}

#[macro_export]
macro_rules! new {
    ($ram:expr, $flash:expr) => {
        mod __buffer {
            use super::*;
            static mut RAM: PIC<[u8; $ram]> = PIC::new([0; $ram]);
            static mut FLASH: PIC<NVM<$flash>> = PIC::new(NVM::new());

            pub fn new() -> SwappingBuffer<'static, 'static, $ram, $flash> {
                unsafe { SwappingBuffer::new(&mut RAM, &mut FLASH) }
            }
        }

        __buffer::new()
    };
}
