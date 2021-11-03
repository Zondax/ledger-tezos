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
use crate::raw::{cx_err_t, exception_t};
//use std::convert::{TryFrom, Infallible};

#[derive( Clone, Copy)]
pub enum SyscallError {
    Code(exception_t),
}

impl From<exception_t> for SyscallError {
    fn from(e: exception_t) -> Self {
        Self::Code(e)
    }
}

impl Into<()> for SyscallError {
    fn into(self) -> () {
        ()
    }
}

impl Into<exception_t> for SyscallError {
    fn into(self) -> exception_t {
        match self {
            Self::Code(e) => e,
        }
    }
}

#[derive( Clone, Copy)]
pub enum CXError {
    Code(cx_err_t),
}

/*
#[derive( Clone, Copy)]
pub enum CXError {
    Carry,
    Locked,
    Unlocked,
    NotLocked,
    NotUnlocked,
    InternalError,
    InvalidParameterSize,
    InvalidParameterValue,
    InvalidParamenter,
    NotInvertible,
    Overflow,
    MemoryFull,
    NoResidue,
}
*/

impl From<cx_err_t> for CXError {
    fn from(e: cx_err_t) -> Self {
        Self::Code(e)
    }
}

impl Into<()> for CXError {
    fn into(self) -> () {
        ()
    }
}

impl Into<cx_err_t> for CXError {
    fn into(self) -> cx_err_t {
        match self {
            Self::Code(e) => e,
        }
    }
}

#[derive( Clone, Copy)]
pub enum Error {
    Syscall(SyscallError),
    Cx(CXError),
}

impl From<SyscallError> for Error {
    fn from(f: SyscallError) -> Self {
        Self::Syscall(f)
    }
}

impl From<CXError> for Error {
    fn from(f: CXError) -> Self {
        Self::Cx(f)
    }
}

impl From<u16> for Error {
    fn from(f: u16) -> Self {
        SyscallError::from(f).into()
    }
}

impl From<u32> for Error {
    fn from(raw: u32) -> Self {
        //if we can convert to u16 and back it means the top u16 is empty
        if raw == (raw as u16) as u32 {
            SyscallError::from(raw as u16).into()
        } else {
            CXError::from(raw).into()
        }
    }
}

impl Into<u32> for Error {
    fn into(self) -> u32 {
        match self {
            Self::Cx(cx) => cx.into(),
            Self::Syscall(sys) => {
                let u: u16 = sys.into();
                u as u32
            },
        }
    }
}


pub fn catch<T, F>(syscall: F) -> Result<T, Error>
where
    F: FnOnce() -> T,
{
    let t = catch_raw(syscall)?;
    Ok(t)
}
