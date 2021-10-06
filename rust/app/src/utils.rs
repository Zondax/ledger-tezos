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
#![allow(dead_code)]

#[macro_export]
#[cfg(test)]
macro_rules! assert_error_code {
    ($tx:expr, $buffer:ident, $expected:expr) => {
        let pos: usize = $tx as _;
        let actual: crate::constants::ApduError = (&$buffer[pos - 2..pos]).try_into().unwrap();
        assert_eq!(actual, $expected);
    };
}

git_testament::git_testament_macros!(git);

pub const GIT_COMMIT_HASH: &str = git_commit_hash!();

pub const BAKING: bool = cfg!(feature = "baking");

mod apdu_wrapper;
pub use apdu_wrapper::*;

mod buffer_upload;
pub use buffer_upload::*;

/// This function returns the index of the first null byte in the slice
#[cfg(test)]
pub fn strlen(s: &[u8]) -> usize {
    let mut count = 0;
    while let Some(&c) = s.get(count) {
        if c == 0 {
            return count;
        }
        count += 1;
    }

    panic!("byte slice did not terminate with null byte, s: {:x?}", s)
}

#[cfg(test)]
mod maybe_null_terminated_to_string {
    use core::str::Utf8Error;
    use std::borrow::ToOwned;
    use std::ffi::{CStr, CString};
    use std::string::String;

    ///This trait is a utility trait to convert a slice of bytes into a CString
    ///
    /// If the string is nul terminated already then no null termination is added
    pub trait MaybeNullTerminatedToString {
        fn to_string_with_check_null(&self) -> Result<String, Utf8Error>;
    }

    impl MaybeNullTerminatedToString for &[u8] {
        fn to_string_with_check_null(&self) -> Result<String, Utf8Error> {
            //attempt to make a cstr first
            if let Ok(cstr) = CStr::from_bytes_with_nul(self) {
                return cstr.to_owned().into_string().map_err(|e| e.utf8_error());
            }

            //in the case above,
            // we could be erroring due to a null byte in the middle
            // or a null byte _missing_ at the end
            //
            //but here we'll error for a null byte at the end or a null byte in the middle
            match CString::new(self.to_vec()) {
                Ok(cstring) => cstring.into_string().map_err(|e| e.utf8_error()),
                Err(err) => {
                    // so with the above error, we can only be erroring here only with a null byte in the middle
                    let nul_pos = err.nul_position();
                    //truncate the string
                    CStr::from_bytes_with_nul(&self[..=nul_pos])
                        //we can't be erroring for a missing null byte at the end,
                        // and also can't error due to a null byte in the middle,
                        // because this is literally the smaller substring to be terminated
                        .unwrap()
                        .to_owned()
                        .into_string()
                        .map_err(|e| e.utf8_error())
                }
            }
        }
    }

    impl<const S: usize> MaybeNullTerminatedToString for [u8; S] {
        fn to_string_with_check_null(&self) -> Result<String, Utf8Error> {
            (&self[..]).to_string_with_check_null()
        }
    }
}

#[cfg(test)]
pub use maybe_null_terminated_to_string::MaybeNullTerminatedToString;
