mod exceptions {
    use crate::raw::{
        jmp_buf, os_longjmp, setjmp, try_context_get, try_context_set, try_context_t,
    };

    cfg_if! {
        if #[cfg(nanox)] {
            include!("errors/exceptionsX.rs");
        } else if #[cfg(nanos)] {
            include!("errors/exceptionsS.rs");
        }
    }

    #[cfg(bolos_sdk)]
    /// General catch mechanism to catch _all_ kinds of exceptions
    ///
    /// As you can see the error type is just a `u32`
    /// the user of `catch` is expected to interptret this value
    pub(crate) fn catch_raw<T, F>(syscall: F) -> Result<T, u32>
    where
        F: FnOnce() -> T,
    {
        let mut result: Option<Result<T, _>> = None;

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
            let exception = context.ex as u32;
            context.ex = 0;
            unsafe {
                try_context_set(context.previous);
            }

            //we got the error so we should return it
            result.replace(Err(exception));
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
    pub fn catch<T, F>(syscall: F) -> Result<T, u32>
    where
        F: FnOnce() -> T,
    {
        Ok(syscall())
    }

    #[cfg(feature = "exception-throw")]
    pub fn throw_raw(exception: u32) -> ! {
        cfg_if! {
            if #[cfg(bolos_sdk)] {
                unsafe {
                    os_longjmp(exception);
                }
                //this should never happen, and it's here for the
                //never type
                unreachable!("returned from longjmp");
            } else {
                panic!("exception = {:?}", exception);
            }
        }
    }
}
pub use exceptions::*;
