//! This module contains all implementation for all High water mark (HWM) things
//!
//! * Handler
//! * Legacy Handler

use crate::{
    constants::{ApduError as Error, APDU_INDEX_INS},
    dispatcher::{
        ApduHandler, INS_LEGACY_QUERY_ALL_HWM, INS_LEGACY_QUERY_MAIN_HWM, INS_LEGACY_RESET,
    },
    sys::{new_wear_leveller, wear_leveller::Wear},
};

use once_cell::unsync::Lazy;

const N_PAGES: usize = 8;

type WearLeveller = Wear<'static, N_PAGES>;

const MAIN_HWM_LEN: usize = 4;
const ALL_HWM_LEN: usize = 12;

// Mainnet Chain ID: NetXdQprcVkpaWU
// types.h:61,0
const MAINNET_CHAIN_ID: u32 = 0x7A06A770;

#[bolos_sys::pic]
static mut MAIN: Lazy<WearLeveller> =
    Lazy::new(|| new_wear_leveller!(N_PAGES).expect("NVM might be corrupted"));

#[bolos_sys::pic]
static mut TEST: Lazy<WearLeveller> =
    Lazy::new(|| new_wear_leveller!(N_PAGES).expect("NVM might be corrupted"));

pub struct LegacyHWM {}

impl LegacyHWM {
    //apdu_baking.c:39,0
    pub fn reset(level: u32) -> Result<(), Error> {
        let wm = WaterMark::reset(level);
        let data: [u8; 52] = wm.into();

        unsafe { MAIN.write(data.clone()) }.map_err(|_| Error::ExecutionError)?;
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

    pub fn format() -> Result<(), Error> {
        unsafe { MAIN.format() }
            .and_then(|_| unsafe { TEST.format() })
            .map_err(|_| Error::ExecutionError)
    }
}

impl ApduHandler for LegacyHWM {
    fn handle(_: &mut u32, tx: &mut u32, _: u32, apdu: &mut [u8]) -> Result<(), Error> {
        match apdu[APDU_INDEX_INS] {
            INS_LEGACY_RESET => {
                let level = {
                    let mut array = [0; 4];
                    array.copy_from_slice(&apdu[5..9]);
                    u32::from_be_bytes(array)
                };

                Self::reset(level)
            }
            INS_LEGACY_QUERY_MAIN_HWM => {
                let payload = Self::hwm()?;
                let len = payload.len();

                if apdu.len() < len {
                    return Err(Error::OutputBufferTooSmall);
                }

                apdu[..len].copy_from_slice(&payload[..]);
                *tx = len as u32;

                Ok(())
            }
            INS_LEGACY_QUERY_ALL_HWM => {
                let payload = Self::all_hwm()?;
                let len = payload.len();

                if apdu.len() < len {
                    return Err(Error::OutputBufferTooSmall);
                }

                apdu[..len].copy_from_slice(&payload[..]);
                *tx = len as u32;

                Ok(())
            }
            _ => Err(Error::InsNotSupported),
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

        Self { endorsement, level }
    }
}

impl Into<[u8; 52]> for WaterMark {
    fn into(self) -> [u8; 52] {
        let mut out = [0; 52];

        let level = self.level.to_be_bytes();
        out[1..5].copy_from_slice(&level[..]);

        out[1] = self.endorsement as _;
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
        let rx = 4 + 1 + 4;
        let mut buffer = [0; 260];

        let reset_level = 420u32;
        let reset_level = reset_level.to_be_bytes();

        //reset state (problematic with other tests)
        LegacyHWM::format().expect("couldn't format");

        buffer[..5].copy_from_slice(&[CLA, INS_LEGACY_RESET, 0, 0, reset_level.len() as u8]);
        buffer[5..rx].copy_from_slice(&reset_level[..]);
        handle_apdu(&mut flags, &mut tx, rx as u32, &mut buffer);

        assert_eq!(tx as usize, 2);
        assert_error_code!(tx, buffer, ApduError::Success);

        let hwm = LegacyHWM::all_hwm().expect("failed retrieving all hwm");
        assert_eq!(&reset_level[..], &hwm[..4]); //main
        assert_eq!(&reset_level[..], &hwm[4..8]); //test
    }

    #[test]
    #[serial(hwm)]
    fn apdu_get_hwm() {
        let mut flags = 0;
        let mut tx = 0;
        let rx = 4;
        let mut buffer = [0; 260];

        let len = MAIN_HWM_LEN;
        //reset state (problematic with other tests)
        LegacyHWM::format().expect("couldn't format");

        //need to write at least once
        LegacyHWM::reset(0).expect("couldn't reset");

        let hwm = LegacyHWM::hwm().expect("failed retrieving hwm");

        buffer[..rx].copy_from_slice(&[CLA, INS_LEGACY_QUERY_MAIN_HWM, 0, 0]);
        handle_apdu(&mut flags, &mut tx, rx as u32, &mut buffer);

        assert_eq!(tx as usize, len + 2);
        assert_error_code!(tx, buffer, ApduError::Success);
        assert_eq!(&buffer[..len], &hwm[..])
    }

    #[test]
    #[serial(hwm)]
    fn apdu_get_hwm_no_write() {
        let mut flags = 0;
        let mut tx = 0;
        let rx = 4;
        let mut buffer = [0; 260];

        //reset state (problematic with other tests)
        LegacyHWM::format().expect("couldn't format");

        let err = LegacyHWM::hwm().expect_err("succeed retrieving hwm");
        assert_eq!(err, ApduError::ExecutionError);

        buffer[..rx].copy_from_slice(&[CLA, INS_LEGACY_QUERY_MAIN_HWM, 0, 0]);
        handle_apdu(&mut flags, &mut tx, rx as u32, &mut buffer);

        assert_eq!(tx as usize, 2);
        assert_error_code!(tx, buffer, ApduError::ExecutionError);
    }

    #[test]
    #[serial(hwm)]
    fn apdu_get_all_hwm() {
        let mut flags = 0;
        let mut tx = 0;
        let rx = 4;
        let mut buffer = [0; 260];

        let len = ALL_HWM_LEN;
        //reset state (problematic with other tests)
        LegacyHWM::format().expect("couldn't format");

        //need to write at least once
        LegacyHWM::reset(0).expect("couldn't reset");

        let hwm = LegacyHWM::all_hwm().expect("failed retrieving all hwm");
        let main = LegacyHWM::hwm().expect("failed retrieving main hwm");

        buffer[..rx].copy_from_slice(&[CLA, INS_LEGACY_QUERY_ALL_HWM, 0, 0]);
        handle_apdu(&mut flags, &mut tx, rx as u32, &mut buffer);

        assert_eq!(tx as usize, len + 2);
        assert_error_code!(tx, buffer, ApduError::Success);
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
}
