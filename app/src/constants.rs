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

// Based on ISO7816
pub enum ApduError {
    ExecutionError = 0x6400,
    WrongLength = 0x6700,
    ApduCodeEmptyBuffer = 0x6982,
    OutputBufferTooSmall = 0x6983,
    DataInvalid = 0x6984,
    ApduCodeConditionsNotSatisfied = 0x6985,
    CommandNotAllowed = 0x6986,
    BadKeyExample = 0x6A80,
    InvalidP1P2 = 0x6B00,
    InsNotSupported = 0x6D00,
    ClaNotSupported = 0x6E00,
    Unknown = 0x6F00,
    SignVerifyError = 0x6F01,
    Success = 0x9000,
    Busy = 0x9001,
}

// FIXME: Convert this to ApduHeader struct
pub const APDU_INDEX_CLA: usize = 0;
pub const APDU_INDEX_INS: usize = 1;
pub const APDU_INDEX_P1: usize = 2;
pub const APDU_INDEX_P2: usize = 3;
pub const APDU_INDEX_LEN: usize = 4;

pub const APDU_MIN_LENGTH: u32 = 5;
