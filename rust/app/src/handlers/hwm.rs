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
    constants::{tzprefix::NET, ApduError as Error},
    sys::{flash_slot::Wear, new_flash_slot, pic::PIC},
    utils::{bs58_encode, ApduPanic},
};

pub use crate::sys::flash_slot::WearError;

use super::sha256x2;

const N_PAGES: usize = 8;

type WearLeveller = Wear<'static, N_PAGES>;

pub const MAIN_HWM_LEN: usize = 4;
pub const ALL_HWM_LEN: usize = 12;

// Mainnet Chain ID: NetXdQprcVkpaWU
// types.h:61,0
pub const MAINNET_CHAIN_ID: u32 = 0x7A06A770;

#[bolos::lazy_static]
static mut MAIN: WearLeveller = new_flash_slot!(N_PAGES).apdu_expect("NVM might be corrupted");

#[bolos::lazy_static]
static mut TEST: WearLeveller = new_flash_slot!(N_PAGES).apdu_expect("NVM might be corrupted");

#[bolos::lazy_static]
static mut CHAIN_ID: WearLeveller = new_flash_slot!(N_PAGES).apdu_expect("NVM might be corrupted");

#[allow(clippy::upper_case_acronyms)]
pub struct HWM;

impl HWM {
    //apdu_baking.c:39,0
    pub fn reset(level: u32) -> Result<(), Error> {
        let wm = WaterMark::reset(level, false);
        let data: [u8; 52] = wm.into();

        unsafe { MAIN.write(data) }.map_err(|_| Error::ExecutionError)?;
        unsafe { TEST.write(data) }.map_err(|_| Error::ExecutionError)?;

        //only override the chain if it's unset
        // so resetting the HWM level doesn't change the chain too
        unsafe {
            if let Err(bolos::flash_slot::WearError::Uninitialized) = CHAIN_ID.read() {
                CHAIN_ID
                    .write(ChainID::from(MAINNET_CHAIN_ID).into())
                    .map_err(|_| Error::ExecutionError)?;
            }
        }

        Ok(())
    }

    //apdu_baking.c:74,0
    pub fn hwm() -> Result<[u8; MAIN_HWM_LEN], WearError> {
        let wm: WaterMark = unsafe { MAIN.read() }?.into();

        Ok(wm.level().to_be_bytes())
    }

    /// Meant to be used for the legacy API
    pub fn hwm_default() -> [u8; MAIN_HWM_LEN] {
        WaterMark::default().level().to_be_bytes()
    }

    pub fn set_chain_id(id: u32) -> Result<(), Error> {
        let mut data = [0; 52];
        data[..4].copy_from_slice(&id.to_be_bytes()[..]);

        unsafe { CHAIN_ID.write(data) }.map_err(|_| Error::ExecutionError)
    }

    pub fn chain_id() -> Result<u32, WearError> {
        let data = unsafe { CHAIN_ID.read() }?;

        let data = arrayref::array_ref!(data, 0, 4);

        Ok(u32::from_be_bytes(*data))
    }

    /// Meant to be used with legacy API
    // why not use MAINNET_CHAIN_ID directly?
    // until we find the actual value returned by default by the legacy app
    // or a good equivalent, then we can just chainge it here
    pub fn chain_id_default() -> u32 {
        MAINNET_CHAIN_ID
    }

    //apdu_baking.c:66,0
    pub fn all_hwm() -> Result<[u8; ALL_HWM_LEN], WearError> {
        let main_wm: WaterMark = unsafe { MAIN.read() }?.into();
        let main_wm = main_wm.level().to_be_bytes();

        let test_wm: WaterMark = unsafe { TEST.read() }?.into();
        let test_wm = test_wm.level().to_be_bytes();

        let chain_id = Self::chain_id()?;

        let mut out = [0; 12];
        out[..4].copy_from_slice(&main_wm[..]);
        out[4..8].copy_from_slice(&test_wm[..]);
        out[8..].copy_from_slice(&chain_id.to_be_bytes()[..]);

        Ok(out)
    }

    /// Meant to be used with legacy API
    pub fn all_hwm_default() -> [u8; ALL_HWM_LEN] {
        let main_wm = WaterMark::default().level().to_be_bytes();
        let test_wm = WaterMark::default().level().to_be_bytes();

        let chain_id = Self::chain_id_default();

        let mut out = [0; 12];
        out[..4].copy_from_slice(&main_wm[..]);
        out[4..8].copy_from_slice(&test_wm[..]);
        out[8..].copy_from_slice(&chain_id.to_be_bytes()[..]);

        out
    }

    #[allow(dead_code)]
    pub fn format() -> Result<(), Error> {
        unsafe { MAIN.format() }
            .and_then(|_| unsafe { TEST.format() })
            .and_then(|_| unsafe { CHAIN_ID.format() })
            .map_err(|_| Error::ExecutionError)
    }

    pub fn write(wm: WaterMark) -> Result<(), Error> {
        let data: [u8; 52] = wm.into();

        unsafe { MAIN.write(data) }.map_err(|_| Error::ExecutionError)
    }

    pub fn write_test(wm: WaterMark) -> Result<(), Error> {
        let data: [u8; 52] = wm.into();

        unsafe { TEST.write(data) }.map_err(|_| Error::ExecutionError)
    }

    pub fn read() -> Result<WaterMark, WearError> {
        let main_wm: WaterMark = unsafe { MAIN.read() }?.into();
        Ok(main_wm)
    }
}

#[derive(PartialEq, Clone)]
#[cfg_attr(test, derive(Debug))]
pub enum WaterMark {
    Emmy {
        level: u32,
        had_endorsement: bool,
    },
    Tenderbake {
        level: u32,
        had_endorsement: bool,
        round: u32,
        had_preendorsement: bool,
    },
}

impl WaterMark {
    const SERIALIZED_EMMY_TAG: u8 = 0;
    const SERIALIZED_TENDERBAKE_TAG: u8 = 1;

    const SERIALIZED_TAGS: PIC<&'static [u8]> =
        PIC::new(&[Self::SERIALIZED_EMMY_TAG, Self::SERIALIZED_TENDERBAKE_TAG]);

    pub fn level(&self) -> u32 {
        match self {
            WaterMark::Emmy { level, .. } | WaterMark::Tenderbake { level, .. } => *level,
        }
    }
}

impl From<&[u8; 52]> for WaterMark {
    fn from(from: &[u8; 52]) -> Self {
        let mut this = Self::default();
        let mut read = 0;

        let ty = from[read];
        read += 1;
        //if the serialization is invalid just return a default
        if !Self::SERIALIZED_TAGS.into_inner().contains(&ty) {
            return this;
        }

        let had_endorsement = from[read] >= 1;
        read += 1;

        let level = {
            let mut array = [0; 4];
            array.copy_from_slice(&from[read..read + 4]);
            u32::from_be_bytes(array)
        };
        read += 4;

        if let Self::SERIALIZED_TENDERBAKE_TAG = ty {
            let had_preendorsement = from[read] >= 1;
            read += 1;

            let round = {
                let mut array = [0; 4];
                array.copy_from_slice(&from[read..read + 4]);
                u32::from_be_bytes(array)
            };

            this = Self::Tenderbake {
                level,
                round,
                had_endorsement,
                had_preendorsement,
            };
        } else {
            this = Self::Emmy {
                level,
                had_endorsement,
            }
        }

        this
    }
}

impl From<WaterMark> for [u8; 52] {
    fn from(from: WaterMark) -> Self {
        let mut out = [0; 52];
        let mut write = 0;

        let serialize_data = |out: &mut [u8], level: u32, had_endorsement| -> usize {
            let mut write = 0;

            out[write] = had_endorsement as _;
            write += 1;

            out[write..write + 4].copy_from_slice(&level.to_be_bytes());
            write += 4;

            write
        };

        match from {
            WaterMark::Emmy {
                level,
                had_endorsement,
            } => {
                out[write] = WaterMark::SERIALIZED_EMMY_TAG;
                write += 1;
                serialize_data(&mut out[write..], level, had_endorsement);
            }
            WaterMark::Tenderbake {
                level,
                had_endorsement,
                round,
                had_preendorsement,
            } => {
                out[write] = WaterMark::SERIALIZED_TENDERBAKE_TAG;
                write += 1;

                write += serialize_data(&mut out[write..], level, had_endorsement);
                serialize_data(&mut out[write..], round, had_preendorsement);
            }
        }

        out
    }
}

impl WaterMark {
    pub fn reset(level: u32, tenderbake: bool) -> Self {
        if tenderbake {
            Self::Tenderbake {
                level,
                round: 0,
                had_endorsement: false,
                had_preendorsement: false,
            }
        } else {
            Self::Emmy {
                level,
                had_endorsement: false,
            }
        }
    }

    //return !(lvl & 0xC0000000);
    #[inline(never)]
    pub fn is_valid_blocklevel(level: u32) -> bool {
        level.leading_zeros() > 0
    }
}

impl Default for WaterMark {
    fn default() -> Self {
        Self::reset(u32::MAX, false)
    }
}

#[derive(PartialEq, Clone, Copy)]
pub enum ChainID {
    Any,
    Mainnet,
    Custom(u32),
}

impl From<&[u8; 52]> for ChainID {
    fn from(from: &[u8; 52]) -> Self {
        let data = arrayref::array_ref!(from, 0, 4);
        let from = u32::from_be_bytes(*data);

        ChainID::from(from)
    }
}

impl From<ChainID> for [u8; 52] {
    fn from(from: ChainID) -> Self {
        let mut out = [0; 52];

        let chain_id: u32 = from.into();
        out[0..4].copy_from_slice(&chain_id.to_be_bytes()[..]);

        out
    }
}

impl From<u32> for ChainID {
    fn from(from: u32) -> Self {
        match from {
            0 => Self::Any,
            MAINNET_CHAIN_ID => Self::Mainnet,
            id => Self::Custom(id),
        }
    }
}

impl From<ChainID> for u32 {
    fn from(from: ChainID) -> Self {
        match from {
            ChainID::Any => 0,
            ChainID::Mainnet => MAINNET_CHAIN_ID,
            ChainID::Custom(n) => n,
        }
    }
}

impl ChainID {
    pub const BASE58_LEN: usize = 16;

    #[inline(never)]
    pub fn id_to_base58(chain_id: u32) -> Result<(usize, [u8; ChainID::BASE58_LEN]), bolos::Error> {
        let mut checksum = [0; 4];
        let chain_id = chain_id.to_be_bytes();

        sha256x2(&[NET, &chain_id[..]], &mut checksum)?;

        let input = {
            let mut array = [0; 3 + 4 + 4];
            array[..3].copy_from_slice(NET);
            array[3..3 + 4].copy_from_slice(&chain_id[..]);
            array[3 + 4..].copy_from_slice(&checksum[..]);
            array
        };

        let mut out = [0; Self::BASE58_LEN];
        let len = bs58_encode(input, &mut out[..])
            .apdu_expect("encoded in base58 is not of the right lenght");

        Ok((len, out))
    }

    pub fn to_alias(self, out: &mut [u8; ChainID::BASE58_LEN]) -> Result<usize, bolos::Error> {
        use bolos::pic_str;

        match self {
            Self::Any => {
                let content = pic_str!(b"any");
                out[..content.len()].copy_from_slice(&content[..]);

                Ok(content.len())
            }
            Self::Mainnet => {
                let content = pic_str!(b"mainnet");
                out[..content.len()].copy_from_slice(&content[..]);

                Ok(content.len())
            }
            Self::Custom(id) => {
                let (len, content) = Self::id_to_base58(id)?;
                out[..len].copy_from_slice(&content[..len]);

                Ok(len)
            }
        }
    }
}
