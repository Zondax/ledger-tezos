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

type WearLeveller = Wear<'static, 'static, N_PAGES>;

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
    pub fn hwm() -> Result<[u8; 4], Error> {
        let wm: WaterMark = unsafe { MAIN.read() }
            .map_err(|_| Error::ExecutionError)?
            .into();

        Ok(wm.level.to_be_bytes())
    }

    //apdu_baking.c:66,0
    pub fn all_hwm() -> Result<[u8; 12], Error> {
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
