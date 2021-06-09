#[derive(Debug, Clone, Copy)]
pub enum SyscallError {
    Code(u16)
}

impl From<u16> for SyscallError {
    fn from(e: u16) -> SyscallError {
        Self::Code(e)
    }
}

impl Into<()> for SyscallError {
    fn into(self) -> () {
        ()
    }
}

impl Into<u16> for SyscallError {
    fn into(self) -> u16 {
        match self {
            SyscallError::Code(e) => e,
        }
    }
}

impl Into<u32> for SyscallError {
    fn into(self) -> u32 {
        let u: u16 = self.into();
        u as u32
    }
}

pub type Error = SyscallError;
