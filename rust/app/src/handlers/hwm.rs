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
//! This module contains all implementation for all High water mark (HWM) things
//!
//! * Handler
//! * Legacy Handler

use crate::{
    constants::{ApduError as Error, APDU_INDEX_INS},
    dispatcher::ApduHandler,
    sys::{flash_slot::Wear, new_flash_slot},
};

const N_PAGES: usize = 8;

type WearLeveller = Wear<'static, N_PAGES>;

const MAIN_HWM_LEN: usize = 4;
const ALL_HWM_LEN: usize = 12;

// Mainnet Chain ID: NetXdQprcVkpaWU
// types.h:61,0
const MAINNET_CHAIN_ID: u32 = 0x7A06A770;

#[bolos::lazy_static]
static mut MAIN: WearLeveller = new_flash_slot!(N_PAGES).expect("NVM might be corrupted");

#[bolos::lazy_static]
static mut TEST: WearLeveller = new_flash_slot!(N_PAGES).expect("NVM might be corrupted");

pub struct LegacyHWM {}

impl LegacyHWM {
    //apdu_baking.c:39,0
    pub fn reset(level: u32) -> Result<(), Error> {
        let wm = WaterMark::reset(level);
        let data: [u8; 52] = wm.into();

        unsafe { MAIN.write(data) }.map_err(|_| Error::ExecutionError)?;
        unsafe { TEST.write(data) }.map_err(|_| Error::ExecutionError)
    }

    //apdu_baking.c:74,0
    pub fn hwm() -> Result<[u8; MAIN_HWM_LEN], Error> {
        let wm: WaterMark = unsafe { MAIN.read() }
            .map_err(|_| Error::ExecutionError)?
            .into();

        Ok(wm.level.to_be_bytes())
    }

    //apdu_baking.c:66,0
    pub fn all_hwm() -> Result<[u8; ALL_HWM_LEN], Error> {
        let main_wm: WaterMark = unsafe { MAIN.read() }
            .map_err(|_| Error::ExecutionError)?
            .into();
        let main_wm = main_wm.level.to_be_bytes();

        let test_wm: WaterMark = unsafe { TEST.read() }
            .map_err(|_| Error::ExecutionError)?
            .into();
        let test_wm = test_wm.level.to_be_bytes();

        let mut out = [0; 12];
        out[..4].copy_from_slice(&main_wm[..]);
        out[4..8].copy_from_slice(&test_wm[..]);
        out[8..].copy_from_slice(&MAINNET_CHAIN_ID.to_be_bytes()[..]);

        Ok(out)
    }

    #[allow(dead_code)]
    pub fn format() -> Result<(), Error> {
        unsafe { MAIN.format() }
            .and_then(|_| unsafe { TEST.format() })
            .map_err(|_| Error::ExecutionError)
    }
}

impl ApduHandler for LegacyHWM {
    #[inline(never)]
    fn handle(_: &mut u32, tx: &mut u32, _: u32, apdu: &mut [u8]) -> Result<(), Error> {
        use crate::dispatcher::{
            INS_LEGACY_QUERY_ALL_HWM, INS_LEGACY_QUERY_MAIN_HWM, INS_LEGACY_RESET,
        };

        let ins = apdu[APDU_INDEX_INS];

        if ins == INS_LEGACY_RESET {
            let level = {
                let mut array = [0; 4];
                array.copy_from_slice(&apdu[5..9]);
                u32::from_be_bytes(array)
            };

            Self::reset(level)
        } else if ins == INS_LEGACY_QUERY_MAIN_HWM {
            let payload = Self::hwm()?;
            let len = payload.len();

            if apdu.len() < len {
                return Err(Error::OutputBufferTooSmall);
            }

            apdu[..len].copy_from_slice(&payload[..]);
            *tx = len as u32;

            Ok(())
        } else if ins == INS_LEGACY_QUERY_ALL_HWM {
            let payload = Self::all_hwm()?;
            let len = payload.len();

            if apdu.len() < len {
                return Err(Error::OutputBufferTooSmall);
            }

            apdu[..len].copy_from_slice(&payload[..]);
            *tx = len as u32;

            Ok(())
        } else {
            Err(Error::InsNotSupported)
        }
    }
}

struct WaterMark {
    level: u32,
    endorsement: bool,
}

impl From<&[u8; 52]> for WaterMark {
    fn from(from: &[u8; 52]) -> Self {
        let endorsement = from[1] >= 1;

        let level = {
            let mut array = [0; 4];
            array.copy_from_slice(&from[1..5]);
            u32::from_be_bytes(array)
        };

        Self { level, endorsement }
    }
}

impl From<WaterMark> for [u8; 52] {
    fn from(from: WaterMark) -> Self {
        let mut out = [0; 52];

        let level = from.level.to_be_bytes();
        out[1..5].copy_from_slice(&level[..]);

        out[1] = from.endorsement as _;
        out
    }
}

impl WaterMark {
    pub fn reset(level: u32) -> Self {
        Self {
            level,
            endorsement: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{LegacyHWM, ALL_HWM_LEN, MAINNET_CHAIN_ID, MAIN_HWM_LEN};
    use crate::{
        assert_error_code,
        constants::ApduError,
        dispatcher::{
            handle_apdu, CLA, INS_LEGACY_QUERY_ALL_HWM, INS_LEGACY_QUERY_MAIN_HWM, INS_LEGACY_RESET,
        },
    };
    use serial_test::serial;
    use std::convert::TryInto;

    #[test]
    #[serial(hwm)]
    fn apdu_reset_hwm() {
        let mut flags = 0;
        let mut tx = 0;
        let rx = 5 + 4;
        let mut buffer = [0; 260];

        let reset_level = 420u32;
        let reset_level = reset_level.to_be_bytes();

        //reset state (problematic with other tests)
        LegacyHWM::format().expect("couldn't format");

        buffer[..5].copy_from_slice(&[CLA, INS_LEGACY_RESET, 0, 0, reset_level.len() as u8]);
        buffer[5..rx].copy_from_slice(&reset_level[..]);
        handle_apdu(&mut flags, &mut tx, rx as u32, &mut buffer);

        assert_error_code!(tx, buffer, ApduError::Success);
        assert_eq!(tx as usize, 2);

        let hwm = LegacyHWM::all_hwm().expect("failed retrieving all hwm");
        assert_eq!(&reset_level[..], &hwm[..4]); //main
        assert_eq!(&reset_level[..], &hwm[4..8]); //test
    }

    #[test]
    #[serial(hwm)]
    fn apdu_get_hwm() {
        let mut flags = 0;
        let mut tx = 0;
        let rx = 5;
        let mut buffer = [0; 260];

        let len = MAIN_HWM_LEN;
        //reset state (problematic with other tests)
        LegacyHWM::format().expect("couldn't format");

        //need to write at least once
        LegacyHWM::reset(0).expect("couldn't reset");

        let hwm = LegacyHWM::hwm().expect("failed retrieving hwm");

        buffer[..rx].copy_from_slice(&[CLA, INS_LEGACY_QUERY_MAIN_HWM, 0, 0, 0]);
        handle_apdu(&mut flags, &mut tx, rx as u32, &mut buffer);

        assert_error_code!(tx, buffer, ApduError::Success);
        assert_eq!(tx as usize, len + 2);
        assert_eq!(&buffer[..len], &hwm[..])
    }

    #[test]
    #[serial(hwm)]
    fn apdu_get_hwm_no_write() {
        let mut flags = 0;
        let mut tx = 0;
        let rx = 5;
        let mut buffer = [0; 260];

        //reset state (problematic with other tests)
        LegacyHWM::format().expect("couldn't format");

        let err = LegacyHWM::hwm().expect_err("succeed retrieving hwm");
        assert_eq!(err, ApduError::ExecutionError);

        buffer[..rx].copy_from_slice(&[CLA, INS_LEGACY_QUERY_MAIN_HWM, 0, 0, 0]);
        handle_apdu(&mut flags, &mut tx, rx as u32, &mut buffer);

        assert_error_code!(tx, buffer, ApduError::ExecutionError);
        assert_eq!(tx as usize, 2);
    }

    #[test]
    #[serial(hwm)]
    fn apdu_get_all_hwm() {
        let mut flags = 0;
        let mut tx = 0;
        let rx = 5;
        let mut buffer = [0; 260];

        let len = ALL_HWM_LEN;
        //reset state (problematic with other tests)
        LegacyHWM::format().expect("couldn't format");

        //need to write at least once
        LegacyHWM::reset(0).expect("couldn't reset");

        let hwm = LegacyHWM::all_hwm().expect("failed retrieving all hwm");
        let main = LegacyHWM::hwm().expect("failed retrieving main hwm");

        buffer[..rx].copy_from_slice(&[CLA, INS_LEGACY_QUERY_ALL_HWM, 0, 0, 0]);
        handle_apdu(&mut flags, &mut tx, rx as u32, &mut buffer);

        assert_error_code!(tx, buffer, ApduError::Success);
        assert_eq!(tx as usize, len + 2);
        assert_eq!(&buffer[..4], &main[..]); //main
        assert_eq!(&buffer[4..8], &main[..]); //main == test in this case because we reset

        let chain_id = {
            let mut array = [0; 4];
            array.copy_from_slice(&buffer[8..12]);
            u32::from_be_bytes(array)
        };
        assert_eq!(chain_id, MAINNET_CHAIN_ID);

        assert_eq!(&buffer[..12], &hwm[..]);
    }

    #[test]
    #[serial(hwm)]
    pub fn trash_01() {
        //reset state (problematic with other tests)
        LegacyHWM::format().expect("couldn't format");

        let req = hex::decode("80060000040000002a").unwrap();

        let (_, tx, out) = crate::handle_apdu_raw(&req);

        std::println!("{:x?}", &out[..tx as usize]);
        assert_error_code!(tx, out, ApduError::Success);
    }
}
