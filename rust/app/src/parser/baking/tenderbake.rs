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
use nom::{
    bytes::complete::{tag, take},
    number::complete::{be_u16, be_u32, le_u8},
    IResult,
};

use crate::handlers::{hwm::WaterMark, parser_common::ParserError};

#[repr(u8)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum EndorsementType {
    PreEndorsement = Self::PREENDORSEMENT_TAG,
    Endorsement = Self::ENDORSEMENT_TAG,
}

impl EndorsementType {
    const PREENDORSEMENT_TAG: u8 = 20;
    const ENDORSEMENT_TAG: u8 = 21;

    pub fn from_tag(tag: u8) -> Option<Self> {
        match tag {
            Self::PREENDORSEMENT_TAG => Some(Self::PreEndorsement),
            Self::ENDORSEMENT_TAG => Some(Self::Endorsement),
            _ => None,
        }
    }
}

pub struct TenderbakeEndorsement<'b> {
    pub chain_id: u32,
    pub branch: &'b [u8; 32],

    pub ty: EndorsementType,
    pub slot: u16,
    pub level: u32,
    pub round: u32,
    pub block_payload_hash: &'b [u8; 32],
}

impl<'b> TenderbakeEndorsement<'b> {
    #[inline(never)]
    pub fn from_bytes(bytes: &'b [u8]) -> IResult<&[u8], Self, ParserError> {
        let (rem, chain_id) = be_u32(bytes)?;
        let (rem, branch) = take(32usize)(rem)?;
        let branch = arrayref::array_ref!(branch, 0, 32);

        let (rem, tag) = le_u8(rem)?;
        let tag = EndorsementType::from_tag(tag).ok_or(ParserError::InvalidEndorsementType)?;

        let (rem, slot) = be_u16(rem)?;
        let (rem, level) = be_u32(rem)?;
        let (rem, round) = be_u32(rem)?;
        let (rem, block_payload_hash) = take(32usize)(rem)?;
        let block_payload_hash = arrayref::array_ref!(block_payload_hash, 0, 32);

        Ok((
            rem,
            Self {
                chain_id,
                branch,
                slot,
                level,
                round,
                ty: tag,
                block_payload_hash,
            },
        ))
    }

    #[inline(never)]
    pub fn validate_with_watermark(&self, hw: &WaterMark) -> bool {
        WaterMark::is_valid_blocklevel(self.level)
            && match *hw {
                //stored watermark is Emmy, and as this is tenderbake
                // we know this is always higher
                WaterMark::Emmy { .. } => true,
                WaterMark::Tenderbake {
                    level,
                    had_endorsement,
                    round,
                    had_preendorsement,
                } => {
                    //1. higher level OR same level with higher round
                    //2. OR, same level, same round, but no endorsement done
                    // and this is an endosement
                    //3. OR, same level, same round, but no preendorsement OR endorsement done
                    // and this is a pre endorsement
                    self.level > level
                        || (self.level == level && self.round > round)
                        || (self.level == level
                            && self.round == round
                            && self.ty == EndorsementType::Endorsement
                            && !had_endorsement)
                        || (self.level == level
                            && self.round == round
                            && self.ty == EndorsementType::PreEndorsement
                            && !had_endorsement
                            && !had_preendorsement)
                }
            }
    }
}

#[derive(Clone)]
#[cfg_attr(test, derive(Debug))]
pub struct TenderbakeFitness<'b> {
    fitness: &'b [u8],
    pub round: u32,
}

impl<'b> TenderbakeFitness<'b> {
    const PROTOCOL_VERSION_TENDERBAKE: u8 = 2;

    #[inline(never)]
    pub fn from_bytes(bytes: &'b [u8]) -> IResult<&[u8], Self, ParserError> {
        let (rem, _proto) = tag(&[Self::PROTOCOL_VERSION_TENDERBAKE])(bytes)?;

        let (_, round) = be_u32(&rem[rem.len() - 4..])?;

        Ok((
            rem,
            Self {
                fitness: rem,
                round,
            },
        ))
    }

    pub fn fitness(&self) -> &[u8] {
        self.fitness
    }
}
