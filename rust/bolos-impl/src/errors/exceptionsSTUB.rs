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
#[derive( Clone, Copy)]
pub enum SyscallError {
    Code(u16)
}

impl From<u16> for SyscallError {
    fn from(e: u16) -> SyscallError {
        Self::Code(e)
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
            SyscallError::Code(e) => e,
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
