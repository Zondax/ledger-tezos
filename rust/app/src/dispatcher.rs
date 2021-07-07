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

use crate::constants::ApduError;
use crate::constants::ApduError::{ClaNotSupported, CommandNotAllowed};

use crate::handlers::public_key::GetAddress;
use crate::handlers::signing::Sign;
use crate::handlers::version::GetVersion;

use crate::handlers::legacy::public_key::{LegacyGetPublic, LegacyPromptAddress};
use crate::handlers::legacy::signing::{LegacySign, LegacySignWithHash};
use crate::handlers::legacy::version::{LegacyGetVersion, LegacyGit};

use crate::utils::ApduBufferRead;

pub const CLA: u8 = 0x80;

//TODO: refactor in an enum
cfg_if! {
    if #[cfg(feature = "baking")] {
        //baking-only legacy instructions
        pub const INS_LEGACY_AUTHORIZE_BAKING: u8 = 0x1;
        pub const INS_LEGACY_RESET: u8 = 0x6;
        pub const INS_LEGACY_QUERY_AUTH_KEY: u8 = 0x7;
        pub const INS_LEGACY_QUERY_MAIN_HWM: u8 = 0x8;
        pub const INS_LEGACY_SETUP: u8 = 0xA;
        pub const INS_LEGACY_QUERY_ALL_HWM: u8 = 0xB;
        pub const INS_LEGACY_DEAUTHORIZE: u8 = 0xC;
        pub const INS_LEGACY_QUERY_AUTH_KEY_WITH_CURVE: u8 = 0xD;
        pub const INS_LEGACY_HMAC: u8 = 0xE;

        pub const INS_AUTHORIZE_BAKING: u8 = 0xA1;
        pub const INS_DEAUTHORIZE_BAKING: u8 = 0xAC;
        pub const INS_QUERY_AUTH_KEY: u8 = 0xA7;
        pub const INS_QUERY_AUTH_KEY_WITH_CURVE: u8 = 0xAD;
        pub const INS_BAKER_SIGN: u8 = 0xAF;

        //baking-only legacy imports
        use crate::handlers::legacy::hwm::{LegacyResetHWM, LegacyQueryMainHWM, LegacyQueryAllHWM};
        use crate::handlers::legacy::baking::{LegacyAuthorize, LegacyDeAuthorize, LegacyQueryAuthKey, LegacyQueryAuthKeyWithCurve};

        //baking-only new instructions
        use crate::handlers::baking::Baking;
    } else if #[cfg(feature = "wallet")] {
        //wallet-only legacy instructions
        pub const INS_LEGACY_SIGN_UNSAFE: u8 = 0x5;

        //wallet-only legacy imports
        use crate::handlers::legacy::signing::LegacySignUnsafe;

        //wallet-only new instructions
    }
}

//common legacy instructions
pub const INS_LEGACY_GET_VERSION: u8 = 0x0;
pub const INS_LEGACY_GET_PUBLIC_KEY: u8 = 0x2;
pub const INS_LEGACY_PROMPT_PUBLIC_KEY: u8 = 0x3;
pub const INS_LEGACY_SIGN: u8 = 0x4;
pub const INS_LEGACY_GIT: u8 = 0x9;
pub const INS_LEGACY_SIGN_WITH_HASH: u8 = 0xF;

//common new instructions
pub const INS_GET_VERSION: u8 = 0x10;
pub const INS_GET_ADDRESS: u8 = 0x11;
pub const INS_SIGN: u8 = 0x12;

//dev-only
cfg_if! {
    if #[cfg(feature = "dev")] {
        use crate::handlers::dev::{Except, Sha256, Echo};

        pub const INS_DEV_HASH: u8 = 0xF0;
        pub const INS_DEV_EXCEPT: u8 = 0xF1;
        pub const INS_DEV_ECHO_UI: u8 = 0xF2;
    }
}

pub trait ApduHandler {
    fn handle<'apdu>(
        flags: &mut u32,
        tx: &mut u32,
        apdu_buffer: ApduBufferRead<'apdu>,
    ) -> Result<(), ApduError>;
}

pub fn apdu_dispatch<'apdu>(
    flags: &mut u32,
    tx: &mut u32,
    apdu_buffer: ApduBufferRead<'apdu>,
) -> Result<(), ApduError> {
    *flags = 0;
    *tx = 0;

    if apdu_buffer.cla() != CLA {
        return Err(ClaNotSupported);
    }

    let ins = apdu_buffer.ins();

    // Reference for legacy API https://github.com/obsidiansystems/ledger-app-tezos/blob/58797b2f9606c5a30dd1ccc9e5b9962e45e10356/src/main.c#L16-L31

    //dev-only instructions
    cfg_if! {
        if #[cfg(feature = "dev")] {
            match ins {
                INS_DEV_HASH => return Sha256::handle(flags, tx, apdu_buffer),
                INS_DEV_EXCEPT => return Except::handle(flags, tx, apdu_buffer),
                INS_DEV_ECHO_UI => return Echo::handle(flags, tx, apdu_buffer),
                _ => {},
            }
        }
    }

    //these are exclusive
    cfg_if! {
        if #[cfg(feature = "baking")] {
            //baking-only instructions
            match ins {
                INS_LEGACY_RESET => return LegacyResetHWM::handle(flags, tx, apdu_buffer),
                INS_LEGACY_QUERY_MAIN_HWM => return LegacyQueryMainHWM::handle(flags, tx, apdu_buffer),
                INS_LEGACY_QUERY_ALL_HWM => return LegacyQueryAllHWM::handle(flags, tx, apdu_buffer),

                INS_AUTHORIZE_BAKING |
                INS_DEAUTHORIZE_BAKING |
                INS_QUERY_AUTH_KEY_WITH_CURVE |
                INS_BAKER_SIGN => return Baking::handle(flags, tx, apdu_buffer),

                INS_LEGACY_AUTHORIZE_BAKING => return LegacyAuthorize::handle(flags, tx, apdu_buffer),
                INS_LEGACY_QUERY_AUTH_KEY => return LegacyQueryAuthKey::handle(flags, tx, apdu_buffer),
                INS_LEGACY_DEAUTHORIZE => return LegacyDeAuthorize::handle(flags, tx, apdu_buffer),
                INS_LEGACY_QUERY_AUTH_KEY_WITH_CURVE => return LegacyQueryAuthKeyWithCurve::handle(flags, tx, apdu_buffer),

                INS_LEGACY_SETUP |
                INS_LEGACY_HMAC => return Err(CommandNotAllowed),
                _ => {}
            }
        } else if #[cfg(feature = "wallet")] {
            //wallet-only instructions
            #[allow(clippy::single_match)]
            match ins {
                INS_LEGACY_SIGN_UNSAFE => return LegacySignUnsafe::handle(flags, tx, apdu_buffer),
                _ => {}
            }
        }
    }

    //common instructions
    // FIXME: Unify using the trait
    match ins {
        INS_LEGACY_GET_VERSION => LegacyGetVersion::handle(flags, tx, apdu_buffer),

        INS_LEGACY_GET_PUBLIC_KEY => LegacyGetPublic::handle(flags, tx, apdu_buffer),
        INS_LEGACY_PROMPT_PUBLIC_KEY => LegacyPromptAddress::handle(flags, tx, apdu_buffer),
        INS_GET_ADDRESS => GetAddress::handle(flags, tx, apdu_buffer),

        INS_LEGACY_GIT => LegacyGit::handle(flags, tx, apdu_buffer),

        INS_LEGACY_SIGN => LegacySign::handle(flags, tx, apdu_buffer),
        INS_LEGACY_SIGN_WITH_HASH => LegacySignWithHash::handle(flags, tx, apdu_buffer),
        INS_SIGN => Sign::handle(flags, tx, apdu_buffer),

        INS_GET_VERSION => GetVersion::handle(flags, tx, apdu_buffer),
        _ => Err(CommandNotAllowed),
    }
}

pub fn handle_apdu(flags: &mut u32, tx: &mut u32, rx: u32, apdu_buffer: &mut [u8]) {
    crate::sys::zemu_log_stack("handle_apdu\x00");

    //construct reader
    let status_word = ApduBufferRead::new(apdu_buffer, rx)
        .map_err(|_| ApduError::WrongLength) //if ther's an error constructing the wrapper, error
        .and_then(|read| apdu_dispatch(flags, tx, read)) //dispatch
        .and(Err::<(), _>(ApduError::Success)) //if we were successfull in dispatch, then it's success
        .map_err(|e| e as u16) //convert to u16
        .unwrap_err(); //get the status

    let txu = *tx as usize;
    apdu_buffer[txu..txu + 2].copy_from_slice(&status_word.to_be_bytes()[..]);

    *tx += 2;
}

#[cfg(test)]
mod tests {
    use crate::assert_error_code;
    use crate::constants::ApduError::WrongLength;
    use crate::dispatcher::handle_apdu;
    use std::convert::TryInto;

    #[test]
    fn apdu_too_short() {
        let flags = &mut 0u32;
        let tx = &mut 0u32;
        let rx = 0u32;
        let buffer = &mut [0u8; 260];

        handle_apdu(flags, tx, rx, buffer);
        assert_eq!(*tx, 2u32);
        assert_error_code!(*tx, buffer, WrongLength);
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
