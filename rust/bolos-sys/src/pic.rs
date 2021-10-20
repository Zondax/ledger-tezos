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
use core::ops::{Deref, DerefMut};

//https://github.com/LedgerHQ/ledger-nanos-sdk/blob/master/src/lib.rs#L179
/// This struct is to be used when dealing with code memory spaces
/// as the memory is mapped differently once the app is installed.
///
/// This struct should then be used when accessing flash memory (via nvm or immutable statics) or
/// function pointers (const in rust is optimized at compile-time)
///
/// # Example
/// ```
/// # use bolos::pic::PIC;
/// //BUFFER is a `static` so we need to wrap it with PIC so it would
/// //be accessible when running under BOLOS
/// #[bolos::pic]
/// static BUFFER: [u8; 1024] = [0; 1024];
///
/// let _: &PIC<[u8; 1024]> = &BUFFER;
/// assert_eq!(&[0; 1024], &*BUFFER);
/// ```
///
/// # Notes on ?Sized types
/// Currently, for every ?Sized type that we need, a separate implementation *has* to be made
///
/// This is because by passing the pointer to C we lose some "fattiness" (for example the length of the item)
/// of the pointer, and we can't manually reconstruct it.
/// If `pic` were ever to be moved to pure rust this limitation could be circumvented.
///
/// An API exists for putting the "fettiness" back, see [Pointee](core::ptr::Pointee),
/// but it's currently unstable
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct PIC<T> {
    data: T,
}

impl<T> PIC<T> {
    pub const fn new(data: T) -> Self {
        Self { data }
    }

    pub fn get_ref(&self) -> &T {
        cfg_if::cfg_if! {
            if #[cfg(bolos_sdk)] {
                let ptr = unsafe { super::raw::pic(&self.data as *const T as _) as *const T };
                unsafe { &*ptr }
            } else {
                &self.data
            }
        }
    }

    /// Warning: this should be used only in conjunction with `nvm_write`
    ///
    /// That's because if you need PIC it means you are accessing
    /// something in the `.text` section, thus you can't write to it normally
    pub fn get_mut(&mut self) -> &mut T {
        cfg_if::cfg_if! {
            if #[cfg(bolos_sdk)] {
                let ptr = unsafe { super::raw::pic(&mut self.data as *mut T as _) as *mut T };

                unsafe { &mut *ptr }
            } else {
                &mut self.data
            }
        }
    }
}

impl<'a, T> PIC<&'a T> {
    pub fn into_inner(self) -> &'a T {
        cfg_if::cfg_if! {
            if #[cfg(bolos_sdk)] {
                let ptr = unsafe { super::raw::pic(self.data as *const T as _) as *const T };

                unsafe {
                    match ptr.as_ref() {
                        //we know it can't be null
                        Some(r) => r,
                        None => core::hint::unreachable_unchecked(),
                    }
                }
            } else {
                self.data
            }
        }
    }
}

impl<'a, T> PIC<&'a mut T> {
    pub fn into_inner(self) -> &'a mut T {
        cfg_if::cfg_if! {
            if #[cfg(bolos_sdk)] {
                let ptr = unsafe { super::raw::pic(self.data as *const T as _) as *mut T };

                unsafe {
                    match ptr.as_mut() {
                        //we know it can't be null
                        Some(r) => r,
                        None => core::hint::unreachable_unchecked(),
                    }
                }
            } else {
                self.data
            }
        }
    }
}

impl<'a> PIC<&'a str> {
    pub fn into_inner(self) -> &'a str {
        cfg_if::cfg_if! {
            if #[cfg(bolos_sdk)] {
                //make use of impl for PIC<&'a [u8]>
                let data = PIC::new(self.data.as_bytes()).into_inner();

                //if this is not utf8 then it's invalid memory
                match core::str::from_utf8(data) {
                    Ok(s) => s,
                    Err(_) => panic!("picced string was garbage")
                }

            } else {
                self.data
            }
        }
    }
}

impl<'a> PIC<&'a [u8]> {
    pub fn into_inner(self) -> &'a [u8] {
        cfg_if::cfg_if! {
            if #[cfg(bolos_sdk)] {
                let data = self.data;
                let data_len = data.len();

                let ptr = unsafe { super::raw::pic(data.as_ptr() as _) as *const u8 };

                let data = unsafe {
                    core::slice::from_raw_parts(ptr, data_len)
                };

                data
            } else {
                self.data
            }
        }
    }
}

impl PIC<()> {
    /// Apply pic manually, interpreting `ptr` as the actual pointer to an _unknwon_ type
    ///
    /// # Safety
    ///
    /// This function is always safe to use, is the output that is dangerous to interpret!
    pub unsafe fn manual(ptr: usize) -> usize {
        cfg_if::cfg_if! {
            if #[cfg(bolos_sdk)] {
                let ptr = super::raw::pic(ptr as _) as usize;

                ptr
            } else {
                ptr
            }
        }
    }
}

impl<T> Deref for PIC<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.get_ref()
    }
}

impl<T> DerefMut for PIC<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.get_mut()
    }
}

impl<T: Default> Default for PIC<T> {
    fn default() -> Self {
        PIC::new(T::default())
    }
}
