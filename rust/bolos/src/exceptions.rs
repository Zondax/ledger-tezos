mod bindings;
use bindings::*;

#[derive(Debug)]
#[repr(u8)]
pub enum SyscallError {
    InvalidParameter = 2,
    Overflow,
    Security,
    InvalidCrc,
    InvalidChecksum,
    InvalidCounter,
    NotSupported,
    InvalidState,
    Timeout,
    Unspecified,
}

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
            _ => SyscallError::Unspecified,
        }
    }
}

pub fn catch_exception<T, E, F>(syscall: F) -> Result<T, E>
where
    E: From<SyscallError>,
    F: FnOnce() -> T,
{
    let mut result: Option<Result<T, E>> = None;

    //BEGIN_TRY
    let mut context = try_context_t::default();
    context.ex = unsafe { setjmp(&mut context.jmp_buf[0] as *mut _) } as u16;

    if context.ex == 0 {
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
