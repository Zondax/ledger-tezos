/*******************************************************************************
*   (c) 2021 Zondax GmbH
*
*  Licensed under the Apache License, Version 2.0 (the "License");
*  you may not use this file except in compliance with the License.
*  You may obtain a copy of the License at
*
*      http://www.apache.org/licenses/LICENSE-2.0
*
*  Unless required by applicable law or agreed to in writing, software
*  distributed under the License is distributed on an "AS IS" BASIS,
*  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
*  See the License for the specific language governing permissions and
*  limitations under the License.
********************************************************************************/
mod exceptions {
    use crate::raw::{os_longjmp, setjmp, try_context_get, try_context_set, try_context_t};

    cfg_if! {
        if #[cfg(nanox)] {
            include!("errors/exceptionsX.rs");
        } else if #[cfg(nanos)] {
            include!("errors/exceptionsS.rs");
        } else if #[cfg(nanosplus)] {
            include!("errors/exceptionsSP.rs");
        }
    }

    /// General catch mechanism to catch _all_ kinds of exceptions
    ///
    /// As you can see the error type is just a `u32`
    /// the user of `catch` is expected to interptret this value
    #[inline(never)]
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
        return match result {
            Some(r) => r,
            None => unsafe { core::hint::unreachable_unchecked() },
        };
    }

    pub fn throw_raw(exception: u32) -> ! {
        unsafe {
            os_longjmp(exception);
        }
        //this should never happen, and it's here for the
        //never type
        unreachable!("returned from longjmp");
    }
}
pub use exceptions::*;
