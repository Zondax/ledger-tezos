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
use bolos::{pic_str, PIC};
use nom::{
    bytes::complete::take,
    number::complete::{be_u32, le_u8},
    IResult,
};
use zemu_sys::ViewError;

use crate::handlers::{handle_ui_message, hwm::WaterMark, parser_common::ParserError};

use super::DisplayableItem;

pub struct EndorsementData<'b> {
    pub chain_id: u32,
    pub branch: &'b [u8; 32],
    pub tag: u8,
    pub level: u32,
}

impl<'b> EndorsementData<'b> {
    #[inline(never)]
    pub fn from_bytes(bytes: &'b [u8]) -> IResult<&[u8], Self, ParserError> {
        let (rem, chain_id) = be_u32(bytes)?;
        let (rem, branch) = take(32usize)(rem)?;
        let branch = arrayref::array_ref!(branch, 0, 32);
        let (rem, tag) = le_u8(rem)?;
        let (rem, level) = be_u32(rem)?;

        Ok((
            rem,
            Self {
                chain_id,
                branch,
                tag,
                level,
            },
        ))
    }

    #[inline(never)]
    pub fn validate_with_watermark(&self, hw: &WaterMark) -> bool {
        WaterMark::is_valid_blocklevel(self.level)
            && (self.level > hw.level || (hw.level == self.level && !hw.endorsement))
    }
}

impl<'b> DisplayableItem for EndorsementData<'b> {
    fn num_items(&self) -> usize {
        1 + 3
    }

    #[inline(never)]
    fn render_item(
        &self,
        item_n: u8,
        title: &mut [u8],
        message: &mut [u8],
        page: u8,
    ) -> Result<u8, ViewError> {
        use lexical_core::{write as itoa, Number};

        match item_n {
            0 => {
                let title_content = pic_str!(b"Type");
                title[..title_content.len()].copy_from_slice(title_content);

                handle_ui_message(&pic_str!(b"Endorsement")[..], message, page)
            }
            1 => {
                let title_content = pic_str!(b"Branch");
                title[..title_content.len()].copy_from_slice(title_content);

                let mut hex_buf = [0; 32 * 2];
                //this is impossible that will error since the sizes are all checked
                hex::encode_to_slice(&self.branch[..], &mut hex_buf).unwrap();

                handle_ui_message(&hex_buf[..], message, page)
            }
            2 => {
                let title_content = pic_str!(b"Blocklevel");
                title[..title_content.len()].copy_from_slice(title_content);

                let mut itoa_buf = [0u8; u32::FORMATTED_SIZE_DECIMAL];

                handle_ui_message(itoa(self.level, &mut itoa_buf), message, page)
            }
            3 => {
                let title_content = pic_str!(b"ChainID");
                title[..title_content.len()].copy_from_slice(title_content);

                let mut itoa_buf = [0u8; u32::FORMATTED_SIZE_DECIMAL];

                handle_ui_message(itoa(self.chain_id, &mut itoa_buf), message, page)
            }
            _ => Err(ViewError::NoData),
        }
    }
}

pub struct BlockData {
    pub chain_id: u32,
    pub level: u32,
    pub proto: u8,
}

impl BlockData {
    #[inline(never)]
    pub fn from_bytes(bytes: &[u8]) -> IResult<&[u8], Self, ParserError> {
        let (rem, chain_id) = be_u32(bytes)?;
        let (rem, level) = be_u32(rem)?;
        let (rem, proto) = le_u8(rem)?;

        Ok((
            rem,
            Self {
                chain_id,
                level,
                proto,
            },
        ))
    }

    #[inline(never)]
    pub fn validate_with_watermark(&self, hw: &WaterMark) -> bool {
        WaterMark::is_valid_blocklevel(self.level) && (self.level > hw.level)
    }
}

impl DisplayableItem for BlockData {
    fn num_items(&self) -> usize {
        1 + 2
    }

    #[inline(never)]
    fn render_item(
        &self,
        item_n: u8,
        title: &mut [u8],
        message: &mut [u8],
        page: u8,
    ) -> Result<u8, ViewError> {
        use lexical_core::{write as itoa, Number};

        match item_n {
            0 => {
                let title_content = pic_str!(b"Type");
                title[..title_content.len()].copy_from_slice(title_content);

                handle_ui_message(&pic_str!(b"Blocklevel")[..], message, page)
            }
            1 => {
                let title_content = pic_str!(b"ChainID");
                title[..title_content.len()].copy_from_slice(title_content);

                let mut itoa_buf = [0u8; u32::FORMATTED_SIZE_DECIMAL];

                handle_ui_message(itoa(self.chain_id, &mut itoa_buf), message, page)
            }
            2 => {
                let title_content = pic_str!(b"Blocklevel");
                title[..title_content.len()].copy_from_slice(title_content);

                let mut itoa_buf = [0u8; u32::FORMATTED_SIZE_DECIMAL];

                handle_ui_message(itoa(self.level, &mut itoa_buf), message, page)
            }
            _ => Err(ViewError::NoData),
        }
    }
}
