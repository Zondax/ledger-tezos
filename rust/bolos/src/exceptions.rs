#[cfg(bolos_sdk)]
mod bindings;
#[cfg(bolos_sdk)]
use bindings::*;

#[derive(Debug)]
pub enum SyscallError {
    InvalidParameter,
    Overflow,
    Security,
    InvalidCrc,
    InvalidChecksum,
    InvalidCounter,
    NotSupported,
    InvalidState,
    Timeout,
    Unspecified(u16),
}

#[cfg(bolos_sdk)]
impl From<exception_t> for SyscallError {
    fn from(e: exception_t) -> SyscallError {
        match e {
            2 => SyscallError::InvalidParameter,
            3 => SyscallError::Overflow,
            4 => SyscallError::Security,
            5 => SyscallError::InvalidCrc,
            6 => SyscallError::InvalidChecksum,
            7 => SyscallError::InvalidCounter,
            8 => SyscallError::NotSupported,
            9 => SyscallError::InvalidState,
            10 => SyscallError::Timeout,
            x => SyscallError::Unspecified(x),
        }
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
            SyscallError::InvalidParameter => 2,
            SyscallError::Overflow => 3,
            SyscallError::Security => 4,
            SyscallError::InvalidCrc => 5,
            SyscallError::InvalidChecksum => 6,
            SyscallError::InvalidCounter => 7,
            SyscallError::NotSupported => 8,
            SyscallError::InvalidState => 9,
            SyscallError::Timeout => 10,
            SyscallError::Unspecified(x) => x,
        }
    }
}

#[cfg(bolos_sdk)]
pub fn catch_exception<E, T, F>(syscall: F) -> Result<T, E>
where
    SyscallError: Into<E>,
    F: FnOnce() -> T,
{
    let mut result: Option<Result<T, E>> = None;

    //BEGIN_TRY
    let mut context = try_context_t::default();
    context.ex = unsafe { setjmp(&mut context.jmp_buf[0] as *mut _) } as u16;

    if context.ex == 0 {
        context.previous = unsafe { try_context_set(&mut context as *mut try_context_t) };
        //TRY
        // this could throw, that means if we actually return
        // then it's an ok
        let val = syscall();

        result.replace(Ok(val));
    } else {
        //CATCH OTHER
        let exception: SyscallError = context.ex.into();
        context.ex = 0;
        unsafe {
            try_context_set(context.previous);
        }

        //we got the error so we should return it
        result.replace(Err(exception.into()));
    }

    //__FINALLYEX
    if unsafe { try_context_get() } == (&mut context as *mut try_context_t) {
        unsafe {
            try_context_set(context.previous);
        }
    }

    //END_TRY
    //should never happen since we catch all exceptions
    if context.ex != 0 {
        unsafe {
            os_longjmp(context.ex as u32);
        }
    }

    //result will be set either way so we can unwrap here
    return result.unwrap();
}

#[cfg(not(bolos_sdk))]
pub fn catch_exception<E, T, F>(syscall: F) -> Result<T, E>
where
    F: FnOnce() -> T,
{
    Ok(syscall())
}

#[cfg(feature = "exception-throw")]
pub fn throw(exception: u16) -> ! {
    cfg_if! {
        if #[cfg(bolos_sdk)] {
            unsafe {
                os_longjmp(exception as u32);
            }
            panic!("returned from longjmp");
        } else {
            panic!("exception = {}", exception);
        }
    }
}
