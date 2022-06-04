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
mod tenderbake;
pub use tenderbake::{EndorsementType, TenderbakeEndorsement};

mod emmy;
pub use emmy::EmmyEndorsement;

use crate::{
    handlers::{handle_ui_message, hwm::WaterMark, parser_common::ParserError},
    utils::ApduPanic,
};
use bolos::{pic_str, PIC};
use zemu_sys::ViewError;

use nom::{
    bytes::complete::take,
    number::complete::{be_u32, be_u64, le_u8},
    IResult,
};

use self::{emmy::EmmyFitness, tenderbake::TenderbakeFitness};

use super::DisplayableItem;

pub struct BlockData<'b> {
    pub chain_id: u32,
    pub level: u32,
    pub proto: u8,
    pub predecessor: &'b [u8; 32],
    pub timestamp: u64,
    pub validation_pass: u8,
    pub operation_hash: &'b [u8; 32],
    pub fitness: Fitness<'b>,
}

impl<'b> BlockData<'b> {
    #[inline(never)]
    pub fn from_bytes(bytes: &'b [u8]) -> IResult<&[u8], Self, ParserError> {
        let (rem, chain_id) = be_u32(bytes)?;
        let (rem, level) = be_u32(rem)?;
        let (rem, proto) = le_u8(rem)?;
        let (rem, predecessor) = take(32usize)(rem)?;
        let predecessor = arrayref::array_ref!(predecessor, 0, 32);

        let (rem, timestamp) = be_u64(rem)?;
        let (rem, validation_pass) = le_u8(rem)?;
        let (rem, operation_hash) = take(32usize)(rem)?;
        let operation_hash = arrayref::array_ref!(operation_hash, 0, 32);

        let (rem, fitness_size) = be_u32(rem)?;
        let (_, fitness) = take(fitness_size)(rem)?;

        let (rem, fitness) = Fitness::from_bytes(fitness)?;

        Ok((
            rem,
            Self {
                chain_id,
                level,
                proto,
                predecessor,
                timestamp,
                validation_pass,
                operation_hash,
                fitness,
            },
        ))
    }

    #[inline(never)]
    pub fn validate_with_watermark(&self, hw: &WaterMark) -> bool {
        WaterMark::is_valid_blocklevel(self.level)
            && match (hw, &self.fitness) {
                (WaterMark::Emmy { level, .. }, Fitness::Emmy(_)) => self.level > *level,
                //the block is invalid if the stored watermark is tenderbake already
                (WaterMark::Tenderbake { .. }, Fitness::Emmy(_)) => false,
                //stored watermark is Emmy, and this is tenderbake
                // so we know this is always higher
                (WaterMark::Emmy { .. }, Fitness::Tenderbake(_)) => true,
                //higher level OR same level with higher round
                (
                    WaterMark::Tenderbake { level, round, .. },
                    Fitness::Tenderbake(TenderbakeFitness {
                        round: self_round, ..
                    }),
                ) => self.level > *level || (self.level == *level && self_round > round),
            }
    }

    pub fn derive_watermark(&self) -> WaterMark {
        match self.fitness {
            Fitness::Emmy(_) => WaterMark::Emmy {
                level: self.level,
                had_endorsement: false,
            },
            Fitness::Tenderbake(TenderbakeFitness { round, .. }) => WaterMark::Tenderbake {
                level: self.level,
                had_endorsement: false,
                round,
                had_preendorsement: false,
            },
        }
    }
}

impl<'b> DisplayableItem for BlockData<'b> {
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

pub enum Fitness<'b> {
    Emmy(EmmyFitness<'b>),
    Tenderbake(TenderbakeFitness<'b>),
}

impl<'b> Fitness<'b> {
    pub fn round(&self) -> u32 {
        match self {
            Fitness::Emmy(_) => 0,
            Fitness::Tenderbake(tb) => tb.round,
        }
    }

    #[inline(never)]
    pub fn from_bytes(bytes: &'b [u8]) -> IResult<&[u8], Self, ParserError> {
        match (
            EmmyFitness::from_bytes(bytes),
            TenderbakeFitness::from_bytes(bytes),
        ) {
            (Ok((rem, fitness)), Err(_)) => Ok((rem, Self::Emmy(fitness))),
            (Err(_), Ok((rem, fitness))) => Ok((rem, Self::Tenderbake(fitness))),
            (Err(_), Err(_)) => Err(ParserError::InvalidProtocolVersion.into()),
            (Ok(_), Ok(_)) => unreachable!(),
        }
    }
}

pub enum EndorsementData<'b> {
    Emmy(EmmyEndorsement<'b>),
    Tenderbake(TenderbakeEndorsement<'b>),
}

impl<'b> EndorsementData<'b> {
    pub fn is_tenderbake(&self) -> bool {
        matches!(self, Self::Tenderbake(_))
    }

    pub fn endorsement_type(&self) -> &'static [u8] {
        match self {
            EndorsementData::Tenderbake(TenderbakeEndorsement {
                ty: EndorsementType::PreEndorsement,
                ..
            }) => pic_str!(b"Preendorsement"),
            _ => pic_str!(b"Endorsement"),
        }
    }

    pub fn branch(&self) -> &[u8; 32] {
        match self {
            EndorsementData::Emmy(EmmyEndorsement { branch, .. })
            | EndorsementData::Tenderbake(TenderbakeEndorsement { branch, .. }) => branch,
        }
    }

    pub fn chain_id(&self) -> u32 {
        match self {
            EndorsementData::Emmy(EmmyEndorsement { chain_id, .. })
            | EndorsementData::Tenderbake(TenderbakeEndorsement { chain_id, .. }) => *chain_id,
        }
    }

    pub fn level(&self) -> u32 {
        match self {
            EndorsementData::Emmy(EmmyEndorsement { level, .. })
            | EndorsementData::Tenderbake(TenderbakeEndorsement { level, .. }) => *level,
        }
    }

    pub fn round(&self) -> Option<u32> {
        match self {
            EndorsementData::Emmy(_) => None,
            EndorsementData::Tenderbake(TenderbakeEndorsement { round, .. }) => Some(*round),
        }
    }

    #[inline(never)]
    pub fn from_bytes(bytes: &'b [u8]) -> IResult<&[u8], Self, ParserError> {
        match EmmyEndorsement::from_bytes(bytes) {
            Ok((rem, emmy)) => Ok((rem, Self::Emmy(emmy))),
            Err(_) => match TenderbakeEndorsement::from_bytes(bytes) {
                Ok((rem, tenderbake)) => Ok((rem, Self::Tenderbake(tenderbake))),
                Err(err) => Err(err),
            },
        }
    }

    pub fn validate_with_watermark(&self, hw: &WaterMark) -> bool {
        match self {
            EndorsementData::Emmy(emmy) => emmy.validate_with_watermark(hw),
            EndorsementData::Tenderbake(tb) => tb.validate_with_watermark(hw),
        }
    }

    pub fn derive_watermark(&self) -> WaterMark {
        match self {
            EndorsementData::Emmy(EmmyEndorsement { level, .. }) => WaterMark::Emmy {
                level: *level,
                had_endorsement: true,
            },
            EndorsementData::Tenderbake(TenderbakeEndorsement {
                level,
                round,
                ty: EndorsementType::Endorsement,
                ..
            }) => WaterMark::Tenderbake {
                level: *level,
                had_endorsement: true,
                round: *round,
                had_preendorsement: true,
            },
            EndorsementData::Tenderbake(TenderbakeEndorsement {
                level,
                round,
                ty: EndorsementType::PreEndorsement,
                ..
            }) => WaterMark::Tenderbake {
                level: *level,
                had_endorsement: false,
                round: *round,
                had_preendorsement: true,
            },
        }
    }
}

impl<'b> DisplayableItem for EndorsementData<'b> {
    fn num_items(&self) -> usize {
        1 + 3 + self.is_tenderbake() as usize
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

                handle_ui_message(self.endorsement_type(), message, page)
            }
            1 => {
                let title_content = pic_str!(b"Branch");
                title[..title_content.len()].copy_from_slice(title_content);

                let mut hex_buf = [0; 32 * 2];
                //this is impossible that will error since the sizes are all checked
                hex::encode_to_slice(&self.branch(), &mut hex_buf).unwrap();

                handle_ui_message(&hex_buf[..], message, page)
            }
            2 => {
                let title_content = pic_str!(b"Blocklevel");
                title[..title_content.len()].copy_from_slice(title_content);

                let mut itoa_buf = [0u8; u32::FORMATTED_SIZE_DECIMAL];

                handle_ui_message(itoa(self.level(), &mut itoa_buf), message, page)
            }
            3 => {
                let title_content = pic_str!(b"ChainID");
                title[..title_content.len()].copy_from_slice(title_content);

                let mut itoa_buf = [0u8; u32::FORMATTED_SIZE_DECIMAL];

                handle_ui_message(itoa(self.chain_id(), &mut itoa_buf), message, page)
            }
            4 if self.is_tenderbake() => {
                let title_content = pic_str!(b"Round");
                title[..title_content.len()].copy_from_slice(title_content);

                let mut itoa_buf = [0u8; u32::FORMATTED_SIZE_DECIMAL];

                handle_ui_message(
                    itoa(self.round().apdu_unwrap(), &mut itoa_buf),
                    message,
                    page,
                )
            }
            _ => Err(ViewError::NoData),
        }
    }
}
