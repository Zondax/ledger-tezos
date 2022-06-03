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
    branch::alt,
    bytes::complete::{tag, take},
    number::complete::be_u32,
    IResult,
};
use zemu_sys::ViewError;

use crate::{
    handlers::{handle_ui_message, hwm::WaterMark, parser_common::ParserError},
    parser::DisplayableItem,
};

pub struct EmmyEndorsement<'b> {
    pub chain_id: u32,
    pub branch: &'b [u8; 32],
    pub level: u32,
}

impl<'b> EmmyEndorsement<'b> {
    const EMMY_ENDORSEMENT_TAG: u8 = 0;

    #[inline(never)]
    pub fn from_bytes(bytes: &'b [u8]) -> IResult<&[u8], Self, ParserError> {
        let (rem, chain_id) = be_u32(bytes)?;
        let (rem, branch) = take(32usize)(rem)?;
        let branch = arrayref::array_ref!(branch, 0, 32);
        let (rem, _) = tag(&[Self::EMMY_ENDORSEMENT_TAG])(rem)?;
        let (rem, level) = be_u32(rem)?;

        Ok((
            rem,
            Self {
                chain_id,
                branch,
                level,
            },
        ))
    }

    #[inline(never)]
    pub fn validate_with_watermark(&self, hw: &WaterMark) -> bool {
        WaterMark::is_valid_blocklevel(self.level)
            && match hw {
                WaterMark::Emmy {
                    level,
                    had_endorsement,
                } => (self.level > *level) || (*level == self.level && !had_endorsement),
                //the consensus is invalid if the stored watermark is tenderbake already
                WaterMark::Tenderbake { .. } => false,
            }
    }
}

pub struct EmmyFitness<'b> {
    fitness: &'b [u8],
}

impl<'b> EmmyFitness<'b> {
    const PROTOCOL_VERSION_EMMY_ZERO_TO_FOUR: u8 = 1;
    const PROTOCOL_VERSION_EMMY_FIVE_TO_ELEVEN: u8 = 2;

    #[inline(never)]
    pub fn from_bytes(bytes: &'b [u8]) -> IResult<&[u8], Self, ParserError> {
        let (rem, _proto) = alt((
            tag(&[Self::PROTOCOL_VERSION_EMMY_ZERO_TO_FOUR]),
            tag(&[Self::PROTOCOL_VERSION_EMMY_FIVE_TO_ELEVEN]),
        ))(bytes)?;

        Ok((rem, Self { fitness: rem }))
    }
}
