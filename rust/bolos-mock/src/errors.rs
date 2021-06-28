#[derive(Debug, Clone, Copy)]
pub enum SyscallError {
    Code(u16),
}

impl From<u16> for SyscallError {
    fn from(e: u16) -> SyscallError {
        Self::Code(e)
    }
}

impl From<SyscallError> for () {
    fn from(_: SyscallError) -> Self {}
}

impl From<SyscallError> for u16 {
    fn from(from: SyscallError) -> Self {
        match from {
            SyscallError::Code(e) => e,
        }
    }
}

impl From<SyscallError> for u32 {
    fn from(from: SyscallError) -> Self {
        u16::from(from) as u32
    }
}

impl From<std::convert::Infallible> for SyscallError {
    fn from(_: std::convert::Infallible) -> Self {
        unsafe { std::hint::unreachable_unchecked() }
    }
}

pub type Error = SyscallError;

pub fn catch<T, F>(syscall: F) -> Result<T, Error>
where
    F: FnOnce() -> T,
{
    Ok(syscall())
}

pub fn throw_raw(exception: u32) -> ! {
    panic!("exception = {:?}", exception);
}
