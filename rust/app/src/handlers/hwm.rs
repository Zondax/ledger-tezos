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
use crate::{
    constants::ApduError as Error,
    sys::{flash_slot::Wear, new_flash_slot},
};

const N_PAGES: usize = 8;

type WearLeveller = Wear<'static, N_PAGES>;

pub const MAIN_HWM_LEN: usize = 4;
pub const ALL_HWM_LEN: usize = 12;

// Mainnet Chain ID: NetXdQprcVkpaWU
// types.h:61,0
pub const MAINNET_CHAIN_ID: u32 = 0x7A06A770;
//TODO: how about other chains?

#[bolos::lazy_static]
static mut MAIN: WearLeveller = new_flash_slot!(N_PAGES).expect("NVM might be corrupted");

#[bolos::lazy_static]
static mut TEST: WearLeveller = new_flash_slot!(N_PAGES).expect("NVM might be corrupted");

pub struct HWM;

impl HWM {
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

    pub fn write(wm: WaterMark) -> Result<(), Error> {
        let data: [u8; 52] = wm.into();

        unsafe { MAIN.write(data) }.map_err(|_| Error::ExecutionError)
    }

    pub fn read() -> Result<WaterMark, Error> {
        let main_wm: WaterMark = unsafe { MAIN.read() }
            .map_err(|_| Error::ExecutionError)?
            .into();
        Ok(main_wm)
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct WaterMark {
    pub level: u32,
    pub endorsement: bool,
}

impl From<&[u8; 52]> for WaterMark {
    fn from(from: &[u8; 52]) -> Self {
        let endorsement = from[0] >= 1;

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

        out[0] = from.endorsement as _;
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

    //return !(lvl & 0xC0000000);
    #[inline(never)]
    pub fn is_valid_blocklevel(level: u32) -> bool {
        level.leading_zeros() > 0
    }
}
