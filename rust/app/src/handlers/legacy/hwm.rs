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
    constants::ApduError as Error, dispatcher::ApduHandler, handlers::hwm::HWM,
    utils::ApduBufferRead,
};

pub struct LegacyResetHWM;
pub struct LegacyQueryMainHWM;
pub struct LegacyQueryAllHWM;

impl ApduHandler for LegacyResetHWM {
    #[inline(never)]
    fn handle<'apdu>(
        _: &mut u32,
        tx: &mut u32,
        buffer: ApduBufferRead<'apdu>,
    ) -> Result<(), Error> {
        *tx = 0;

        let payload = buffer.payload().map_err(|_| Error::DataInvalid)?;

        let level = {
            let mut array = [0; 4];
            array.copy_from_slice(&payload[..4]);
            u32::from_be_bytes(array)
        };

        HWM::reset(level)
    }
}

impl ApduHandler for LegacyQueryMainHWM {
    #[inline(never)]
    fn handle<'apdu>(
        _: &mut u32,
        tx: &mut u32,
        buffer: ApduBufferRead<'apdu>,
    ) -> Result<(), Error> {
        *tx = 0;

        let hwm = HWM::hwm()?;
        let len = hwm.len();

        let buffer = buffer.write();
        if buffer.len() < len {
            return Err(Error::OutputBufferTooSmall);
        }

        buffer[..len].copy_from_slice(&hwm[..]);
        *tx = len as u32;

        Ok(())
    }
}

impl ApduHandler for LegacyQueryAllHWM {
    #[inline(never)]
    fn handle<'apdu>(
        _: &mut u32,
        tx: &mut u32,
        buffer: ApduBufferRead<'apdu>,
    ) -> Result<(), Error> {
        *tx = 0;

        let hwm = HWM::all_hwm()?;
        let len = hwm.len();

        let buffer = buffer.write();
        if buffer.len() < len {
            return Err(Error::OutputBufferTooSmall);
        }

        buffer[..len].copy_from_slice(&hwm[..]);
        *tx = len as u32;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        assert_error_code,
        constants::ApduError,
        dispatcher::{
            handle_apdu, CLA, INS_LEGACY_QUERY_ALL_HWM, INS_LEGACY_QUERY_MAIN_HWM, INS_LEGACY_RESET,
        },
        handlers::hwm::*,
    };
    use serial_test::serial;
    use std::convert::TryInto;

    #[test]
    fn test_watermark() {
        let mut bytes_init = [0xff; 52];
        bytes_init[0] = 0u8;
        let hw = WaterMark::from(&bytes_init);
        let bytes: [u8; 52] = hw.clone().into();
        let hw2 = WaterMark::from(&bytes);
        assert_eq!(hw, hw2);
    }

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
        HWM::format().expect("couldn't format");

        buffer[..5].copy_from_slice(&[CLA, INS_LEGACY_RESET, 0, 0, reset_level.len() as u8]);
        buffer[5..rx].copy_from_slice(&reset_level[..]);
        handle_apdu(&mut flags, &mut tx, rx as u32, &mut buffer);

        assert_error_code!(tx, buffer, ApduError::Success);
        assert_eq!(tx as usize, 2);

        let hwm = HWM::all_hwm().expect("failed retrieving all hwm");
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
        HWM::format().expect("couldn't format");

        //need to write at least once
        HWM::reset(0).expect("couldn't reset");

        let hwm = HWM::hwm().expect("failed retrieving hwm");

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
        HWM::format().expect("couldn't format");

        let err = HWM::hwm().expect_err("succeed retrieving hwm");
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
        HWM::format().expect("couldn't format");

        //need to write at least once
        HWM::reset(0).expect("couldn't reset");

        let hwm = HWM::all_hwm().expect("failed retrieving all hwm");
        let main = HWM::hwm().expect("failed retrieving main hwm");

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
        HWM::format().expect("couldn't format");

        let req = hex::decode("80060000040000002a").unwrap();

        let (_, tx, out) = crate::handle_apdu_raw(&req);

        std::println!("{:x?}", &out[..tx as usize]);
        assert_error_code!(tx, out, ApduError::Success);
    }
}
