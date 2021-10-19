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
use crate::raw::exception_t;
use std::convert::TryFrom;

#[derive( Clone, Copy)]
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

impl TryFrom<exception_t> for SyscallError {
    type Error = ();

    fn try_from(e: exception_t) -> Result<SyscallError, ()> {
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

impl Into<exception_t> for SyscallError {
    fn into(self) -> exception_t {
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

impl Into<u32> for SyscallError {
    fn into(self) -> u32 {
        let u: u16 = self.into();
        u as u32
    }
}

pub type Error = SyscallError;

pub fn catch<T, F>(syscall: F) -> Result<T, Error>
where
    F: FnOnce() -> T,
{
    match catch_raw(syscall) {
        Ok(t) => Ok(t),
        Err(raw) => Err(Error::try_from(raw as u16).unwrap_or(Error::Exception)),
    }
}
