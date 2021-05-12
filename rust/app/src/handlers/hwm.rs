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

#[bolos_sys::pic]
static mut MAIN: Lazy<WearLeveller> =
    Lazy::new(|| new_wear_leveller!(N_PAGES).expect("NVM might be corrupted"));

#[bolos_sys::pic]
static mut TEST: Lazy<WearLeveller> =
    Lazy::new(|| new_wear_leveller!(N_PAGES).expect("NVM might be corrupted"));

pub struct LegacyHWM {}

impl LegacyHWM {
    pub fn reset() -> Result<(), Error> {
        todo!()
    }

    pub fn hwm() -> Result<(), Error> {
        todo!()
    }

    pub fn all_hwm() -> Result<(), Error> {
        todo!()
    }
}

impl ApduHandler for LegacyHWM {
    fn handle(_: &mut u32, _tx: &mut u32, _: u32, apdu: &mut [u8]) -> Result<(), Error> {
        match apdu[APDU_INDEX_INS] {
            INS_LEGACY_RESET => Self::reset(),
            INS_LEGACY_QUERY_MAIN_HWM => Self::hwm(),
            INS_LEGACY_QUERY_ALL_HWM => Self::all_hwm(),
            _ => Err(Error::InsNotSupported),
        }
    }
}

struct WaterMark {
    level: u32,
    endorsement: bool,
}

impl From<[u8; 52]> for WaterMark {
    fn from(from: [u8; 52]) -> Self {
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
