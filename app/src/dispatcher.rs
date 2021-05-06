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

use cfg_if::cfg_if;

use crate::constants::ApduError::{ClaNotSupported, CommandNotAllowed, Success, WrongLength};
use crate::constants::{ApduError, APDU_INDEX_CLA, APDU_INDEX_INS, APDU_MIN_LENGTH};
use crate::handlers::legacy_public_key::LegacyGetPublicKey;
use crate::handlers::legacy_sign::LegacySign;
use crate::handlers::legacy_version::{LegacyGetVersion, LegacyGit};
use crate::handlers::version::GetVersion;

pub const CLA: u8 = 0x80;

pub const INS_LEGACY_GET_VERSION: u8 = 0x0;
pub const INS_LEGACY_AUTHORIZE_BAKING: u8 = 0x1;
pub const INS_LEGACY_GET_PUBLIC_KEY: u8 = 0x2;
pub const INS_LEGACY_PROMPT_PUBLIC_KEY: u8 = 0x3;
pub const INS_LEGACY_SIGN: u8 = 0x4;
pub const INS_LEGACY_SIGN_UNSAFE: u8 = 0x5;
pub const INS_LEGACY_RESET: u8 = 0x6;
pub const INS_LEGACY_QUERY_AUTH_KEY: u8 = 0x7;
pub const INS_LEGACY_QUERY_MAIN_HWM: u8 = 0x8;
pub const INS_LEGACY_GIT: u8 = 0x9;
pub const INS_LEGACY_SETUP: u8 = 0xA;
pub const INS_LEGACY_QUERY_ALL_HWM: u8 = 0xB;
pub const INS_LEGACY_DEAUTHORIZE: u8 = 0xC;
pub const INS_LEGACY_QUERY_AUTH_KEY_WITH_CURVE: u8 = 0xD;
pub const INS_LEGACY_HMAC: u8 = 0xE;
pub const INS_LEGACY_SIGN_WITH_HASH: u8 = 0xF;

pub const INS_GET_VERSION: u8 = 0x10;
pub const INS_GET_ADDRESS: u8 = 0x11;
pub const INS_SIGN: u8 = 0x12;

cfg_if! {
    if #[cfg(feature = "dev")] {
        use crate::handlers::dev::Dev;

        pub const INS_DEV_HASH: u8 = 0xF0;
    }
}

pub trait ApduHandler {
    fn handle(
        _flags: &mut u32,
        tx: &mut u32,
        _rx: u32,
        apdu_buffer: &mut [u8],
    ) -> Result<(), ApduError>;
}

pub fn apdu_dispatch(
    flags: &mut u32,
    tx: &mut u32,
    rx: u32,
    apdu_buffer: &mut [u8],
) -> Result<(), ApduError> {
    *flags = 0;
    *tx = 0;

    if rx < APDU_MIN_LENGTH {
        return Err(WrongLength);
    }

    if apdu_buffer[APDU_INDEX_CLA] != CLA {
        return Err(ClaNotSupported);
    }

    let ins = apdu_buffer[APDU_INDEX_INS];

    // Reference for legacy API https://github.com/obsidiansystems/ledger-app-tezos/blob/58797b2f9606c5a30dd1ccc9e5b9962e45e10356/src/main.c#L16-L31

    cfg_if! {
        if #[cfg(feature = "dev")] {
            match ins {
                INS_DEV_HASH => return Dev::handle(flags, tx, rx, apdu_buffer),
                _ => {}
            }
        }
    }

    // FIXME: Unify using the trait
    match ins {
        INS_LEGACY_GET_VERSION => LegacyGetVersion::handle(flags, tx, rx, apdu_buffer),

        INS_LEGACY_GET_PUBLIC_KEY => LegacyGetPublicKey::handle(flags, tx, rx, apdu_buffer),
        INS_LEGACY_PROMPT_PUBLIC_KEY => LegacyGetPublicKey::handle(flags, tx, rx, apdu_buffer),

        INS_LEGACY_GIT => LegacyGit::handle(flags, tx, rx, apdu_buffer),

        INS_LEGACY_SIGN => LegacySign::handle(flags, tx, rx, apdu_buffer),
        INS_LEGACY_SIGN_WITH_HASH => LegacySign::handle(flags, tx, rx, apdu_buffer),
        INS_LEGACY_SIGN_UNSAFE => LegacySign::handle(flags, tx, rx, apdu_buffer),

        INS_LEGACY_AUTHORIZE_BAKING => Err(CommandNotAllowed),
        INS_LEGACY_RESET => Err(CommandNotAllowed),
        INS_LEGACY_QUERY_AUTH_KEY => Err(CommandNotAllowed),
        INS_LEGACY_QUERY_MAIN_HWM => Err(CommandNotAllowed),
        INS_LEGACY_SETUP => Err(CommandNotAllowed),
        INS_LEGACY_QUERY_ALL_HWM => Err(CommandNotAllowed),
        INS_LEGACY_DEAUTHORIZE => Err(CommandNotAllowed),
        INS_LEGACY_QUERY_AUTH_KEY_WITH_CURVE => Err(CommandNotAllowed),
        INS_LEGACY_HMAC => Err(CommandNotAllowed),

        INS_GET_VERSION => GetVersion::handle(flags, tx, rx, apdu_buffer),
        _ => Err(CommandNotAllowed),
    }
}

pub fn handle_apdu(flags: &mut u32, tx: &mut u32, rx: u32, apdu_buffer: &mut [u8]) {
    let response = apdu_dispatch(flags, tx, rx, apdu_buffer);

    // Retrieve error code or use 0x9000 if ok
    let error_bytes: [u8; 2] = response
        .map_or_else(|e: ApduError| e as u16, |_| Success as u16)
        .to_be_bytes();
    let error_position = *tx as usize;

    // Copy error code at the end of the response
    apdu_buffer[error_position..error_position + 2].clone_from_slice(&error_bytes);
    *tx += 2;
}

#[cfg(test)]
mod tests {
    use crate::constants::ApduError::WrongLength;
    use crate::dispatcher::handle_apdu;
    use crate::utils::assert_error_code;

    #[test]
    fn apdu_too_short() {
        let flags = &mut 0u32;
        let tx = &mut 0u32;
        let rx = 0u32;
        let buffer = &mut [0u8; 260];

        handle_apdu(flags, tx, rx, buffer);
        assert_eq!(*tx, 2u32);
        assert_error_code(tx, buffer, WrongLength);
    }

    #[test]
    fn apdu_invalid_cla() {
        let flags = &mut 0u32;
        let tx = &mut 0u32;
        let rx = 5u32;
        let buffer = &mut [0u8; 260];

        handle_apdu(flags, tx, rx, buffer);
        assert_eq!(*tx, 2u32);
    }
}
