#[cfg(bolos_sdk)]
mod bindings;
use std::convert::TryFrom;

#[cfg(bolos_sdk)]
use bindings::*;

//////----- exception_t === u16
#[cfg(bolos_sdk)]
type Exception = exception_t;

#[cfg(not(bolos_sdk))]
type Exception = u16;
//////

#[derive(Debug)]
pub enum SyscallError {
    Exception,
    InvalidParameter,
    Overflow,
    Security,
    InvalidCrc,
    InvalidChecksum,
    InvalidCounter,
    NotSupported,
    InvalidState,
    Timeout,
    PIC,
    Appexit,
    IoOverflow,
    IoHeader,
    IoState,
    IoReset,
    CXPort,
    System,
    NotEnoughSpace,
}

impl TryFrom<Exception> for SyscallError {
    type Error = ();

    fn try_from(e: Exception) -> Result<SyscallError, ()> {
        match e {
            1 => Ok(Self::Exception),
            2 => Ok(Self::InvalidParameter),
            3 => Ok(Self::Overflow),
            4 => Ok(Self::Security),
            5 => Ok(Self::InvalidCrc),
            6 => Ok(Self::InvalidChecksum),
            7 => Ok(Self::InvalidCounter),
            8 => Ok(Self::NotSupported),
            9 => Ok(Self::InvalidState),
            10 => Ok(Self::Timeout),
            11 => Ok(Self::PIC),
            12 => Ok(Self::Appexit),
            13 => Ok(Self::IoOverflow),
            14 => Ok(Self::IoHeader),
            15 => Ok(Self::IoState),
            16 => Ok(Self::CXPort),
            17 => Ok(Self::System),
            18 => Ok(Self::NotEnoughSpace),
            _ => Err(()),
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
            SyscallError::Exception => 1,
            SyscallError::InvalidParameter => 2,
            SyscallError::Overflow => 3,
            SyscallError::Security => 4,
            SyscallError::InvalidCrc => 5,
            SyscallError::InvalidChecksum => 6,
            SyscallError::InvalidCounter => 7,
            SyscallError::NotSupported => 8,
            SyscallError::InvalidState => 9,
            SyscallError::Timeout => 10,
            SyscallError::PIC => 11,
            SyscallError::Appexit => 12,
            SyscallError::IoOverflow => 13,
            SyscallError::IoHeader => 14,
            SyscallError::IoState => 15,
            SyscallError::IoReset => 16,
            SyscallError::CXPort => 17,
            SyscallError::System => 18,
            SyscallError::NotEnoughSpace => 19,
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
        //we provide a default in case the conversion fails, but that should never happen
        //except if our mapping from C is incomplete
        //(from rust it would need unsafe since we can't throw directly, but must go
        //thru SyscallError first)
        let exception = SyscallError::try_from(context.ex).unwrap_or(SyscallError::Exception);
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
pub fn throw(exception: SyscallError) -> ! {
    cfg_if! {
        if #[cfg(bolos_sdk)] {
            unsafe {
                let exception: u16 = exception.into();
                os_longjmp(exception as u32);
            }
            //this should never happen, and it's here for the
            //never type
            unreachable!("returned from longjmp");
        } else {
            panic!("exception = {:?}", exception);
        }
    }
}
