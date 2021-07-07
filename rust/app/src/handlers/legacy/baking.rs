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
use crate::{constants::ApduError as Error, dispatcher::ApduHandler, utils::ApduBufferRead};

pub struct LegacyAuthorize;
pub struct LegacyDeAuthorize;
pub struct LegacyQueryAuthKey;
pub struct LegacyQueryAuthKeyWithCurve;

impl ApduHandler for LegacyAuthorize {
    #[inline(never)]
    fn handle<'apdu>(_: &mut u32, _: &mut u32, _: ApduBufferRead<'apdu>) -> Result<(), Error> {
        Err(Error::CommandNotAllowed)
    }
}

impl ApduHandler for LegacyDeAuthorize {
    #[inline(never)]
    fn handle<'apdu>(_: &mut u32, _: &mut u32, _: ApduBufferRead<'apdu>) -> Result<(), Error> {
        Err(Error::CommandNotAllowed)
    }
}

impl ApduHandler for LegacyQueryAuthKey {
    #[inline(never)]
    fn handle<'apdu>(_: &mut u32, _: &mut u32, _: ApduBufferRead<'apdu>) -> Result<(), Error> {
        Err(Error::CommandNotAllowed)
    }
}

impl ApduHandler for LegacyQueryAuthKeyWithCurve {
    #[inline(never)]
    fn handle<'apdu>(_: &mut u32, _: &mut u32, _: ApduBufferRead<'apdu>) -> Result<(), Error> {
        Err(Error::CommandNotAllowed)
    }
}
