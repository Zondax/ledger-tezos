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
use arrayref::array_ref;
use nom::{
    bytes::complete::take,
    number::complete::{be_i32, be_u16, be_u32, be_u8},
    IResult,
};
use zemu_sys::ViewError;

use crate::{
    handlers::{handle_ui_message, parser_common::ParserError},
    parser::DisplayableItem,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, property::Property)]
#[property(mut(disable), get(public), set(disable))]
pub struct Endorsement {
    level: i32,
}

impl Endorsement {
    pub fn from_bytes(input: &[u8]) -> IResult<&[u8], Self, ParserError> {
        let (rem, level) = be_i32(input)?;

        Ok((rem, Self { level }))
    }
}

impl DisplayableItem for Endorsement {
    fn num_items(&self) -> usize {
        1 + 1
    }

    #[inline(never)]
    fn render_item(
        &self,
        item_n: u8,
        title: &mut [u8],
        message: &mut [u8],
        page: u8,
    ) -> Result<u8, ViewError> {
        use bolos::{pic_str, PIC};
        use lexical_core::{write as itoa, Number};

        match item_n {
            //Homepage
            0 => {
                let title_content = pic_str!(b"Type");
                title[..title_content.len()].copy_from_slice(title_content);

                handle_ui_message(&pic_str!(b"Endorsement")[..], message, page)
            }
            //Level
            1 => {
                let title_content = pic_str!(b"Level");
                title[..title_content.len()].copy_from_slice(title_content);

                let mut zarith_buf = [0; i32::FORMATTED_SIZE_DECIMAL];

                handle_ui_message(itoa(self.level, &mut zarith_buf), message, page)
            }
            _ => Err(ViewError::NoData),
        }
    }
}

#[cfg(test)]
impl Endorsement {
    pub fn is(&self, json: &serde_json::Map<std::string::String, serde_json::Value>) {
        let expected = json["level"]
            .as_i64()
            .expect("given json .level is not a signed integer");

        assert_eq!(self.level, expected as i32);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, property::Property)]
#[property(mut(disable), get(public), set(disable))]
pub struct EndorsementWithSlot<'b> {
    branch: &'b [u8; 32],
    endorsement: Endorsement,
    signature: &'b [u8],
    slot: u16,
}

impl<'b> EndorsementWithSlot<'b> {
    pub fn from_bytes(input: &'b [u8]) -> IResult<&[u8], Self, ParserError> {
        let (rem, length) = be_u32(input)?;
        let (rem, branch) = {
            let (rem, branch) = take(32usize)(rem)?;
            (rem, array_ref!(branch, 0, 32))
        };
        let (rem, endorsement_tag) = be_u8(rem)?;
        if endorsement_tag != 0x00 {
            return Err(ParserError::parser_invalid_transaction_payload.into());
        }

        let (rem2, endorsement) = Endorsement::from_bytes(rem)?;
        let length = (length as usize) - 32 - 1 - (rem.len() - rem2.len());
        let (rem, sig) = take(length)(rem2)?;
        let (rem, slot) = be_u16(rem)?;

        Ok((
            rem,
            Self {
                branch,
                endorsement,
                signature: sig,
                slot,
            },
        ))
    }
}

impl<'b> DisplayableItem for EndorsementWithSlot<'b> {
    fn num_items(&self) -> usize {
        1 + 2 + self.endorsement.num_items()
        //TODO: show signature
        // + 1
    }

    #[inline(never)]
    fn render_item(
        &self,
        item_n: u8,
        title: &mut [u8],
        message: &mut [u8],
        page: u8,
    ) -> Result<u8, ViewError> {
        use bolos::{pic_str, PIC};
        use lexical_core::{write as itoa, Number};

        match item_n {
            //Homepage
            0 => {
                let title_content = pic_str!(b"Type");
                title[..title_content.len()].copy_from_slice(title_content);

                handle_ui_message(&pic_str!(b"Endorsement")[..], message, page)
            }
            //Branch
            1 => {
                let title_content = pic_str!(b"Branch");
                title[..title_content.len()].copy_from_slice(title_content);

                let branch =
                    super::Operation::base58_branch(self.branch).map_err(|_| ViewError::Unknown)?;

                handle_ui_message(&branch[..], message, page)
            }
            //Slot
            2 => {
                let title_content = pic_str!(b"Slot");
                title[..title_content.len()].copy_from_slice(title_content);

                let mut mex = [0; u16::FORMATTED_SIZE_DECIMAL];

                handle_ui_message(itoa(self.slot, &mut mex), message, page)
            }
            //TODO: Signature
            // 3 => {}
            n => self.endorsement.render_item(n - 3, title, message, page),
        }
    }
}

#[cfg(test)]
impl<'b> EndorsementWithSlot<'b> {
    pub fn is(&self, json: &serde_json::Map<std::string::String, serde_json::Value>) {
        let branch_base58 =
            super::Operation::base58_branch(self.branch).expect("couldn't compute branch base58");
        let expected_branch_base58 = json["branch"]
            .as_str()
            .expect("given json .branch is not a string");
        assert_eq!(branch_base58, expected_branch_base58.as_bytes());

        let expected = json["slot"]
            .as_i64()
            .expect("given json .slot is not a signed integer");

        assert_eq!(self.slot, expected as u16);

        self.endorsement.is(json)

        //TODO: verify signature
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, property::Property)]
#[property(mut(disable), get(public), set(disable))]
pub struct DoubleEndorsementEvidence<'b> {
    first_branch: &'b [u8; 32],
    first_endorsement: Endorsement,
    first_signature: &'b [u8],
    second_branch: &'b [u8; 32],
    second_endorsement: Endorsement,
    second_signature: &'b [u8],
    slot: u16,
}

impl<'b> DoubleEndorsementEvidence<'b> {
    pub fn from_bytes(input: &'b [u8]) -> IResult<&[u8], Self, ParserError> {
        let (rem, length) = be_u32(input)?;
        let (rem, first_branch) = {
            let (rem, branch) = take(32usize)(rem)?;
            (rem, array_ref!(branch, 0, 32))
        };
        let (rem, endorsement_tag) = be_u8(rem)?;
        if endorsement_tag != 0x00 {
            return Err(ParserError::parser_invalid_transaction_payload.into());
        }

        let (rem2, first_endorsement) = Endorsement::from_bytes(rem)?;
        let length = (length as usize) - 32 - 1 - (rem.len() - rem2.len());
        let (rem, first_signature) = take(length)(rem2)?;

        // --------- Second endorsement

        let (rem, length) = be_u32(rem)?;
        let (rem, second_branch) = {
            let (rem, branch) = take(32usize)(rem)?;
            (rem, array_ref!(branch, 0, 32))
        };
        let (rem, endorsement_tag) = be_u8(rem)?;
        if endorsement_tag != 0x00 {
            return Err(ParserError::parser_invalid_transaction_payload.into());
        }

        let (rem2, second_endorsement) = Endorsement::from_bytes(rem)?;
        let length = (length as usize) - 32 - 1 - (rem.len() - rem2.len());
        let (rem, second_signature) = take(length)(rem2)?;

        let (rem, slot) = be_u16(rem)?;

        Ok((
            rem,
            Self {
                first_branch,
                first_endorsement,
                first_signature,
                second_branch,
                second_endorsement,
                second_signature,
                slot,
            },
        ))
    }
}

impl<'b> DisplayableItem for DoubleEndorsementEvidence<'b> {
    fn num_items(&self) -> usize {
        1 + 2 + self.first_endorsement.num_items() + 2 + self.second_endorsement.num_items()
        //TODO: show signatures
        // + 2
    }

    #[inline(never)]
    fn render_item(
        &self,
        item_n: u8,
        title: &mut [u8],
        message: &mut [u8],
        page: u8,
    ) -> Result<u8, ViewError> {
        use bolos::{pic_str, PIC};
        use lexical_core::{write as itoa, Number};

        let first_items_range = 3 + self.first_endorsement.num_items() as u8;
        let second_items_range = first_items_range + 1 + self.second_endorsement.num_items() as u8;

        match item_n {
            //Homepage
            0 => {
                let title_content = pic_str!(b"Type");
                title[..title_content.len()].copy_from_slice(title_content);

                handle_ui_message(&pic_str!(b"Endorsement")[..], message, page)
            }
            //Slot
            1 => {
                let title_content = pic_str!(b"Slot");
                title[..title_content.len()].copy_from_slice(title_content);

                let mut mex = [0; u16::FORMATTED_SIZE_DECIMAL];

                handle_ui_message(itoa(self.slot, &mut mex), message, page)
            }
            //Branch 1
            2 => {
                let title_content = pic_str!(b"First Branch");
                title[..title_content.len()].copy_from_slice(title_content);

                let branch = super::Operation::base58_branch(self.first_branch)
                    .map_err(|_| ViewError::Unknown)?;

                handle_ui_message(&branch[..], message, page)
            }
            //TODO: Signature
            // 2 => {}
            n if n < first_items_range => {
                self.first_endorsement
                    .render_item(n - 3, title, message, page)
            }
            //Branch 2
            n if n >= first_items_range && n < first_items_range + 1 => {
                let title_content = pic_str!(b"Second Branch");
                title[..title_content.len()].copy_from_slice(title_content);

                let branch = super::Operation::base58_branch(self.second_branch)
                    .map_err(|_| ViewError::Unknown)?;

                handle_ui_message(&branch[..], message, page)
            }
            n if n < second_items_range => self.second_endorsement.render_item(
                n - (first_items_range + 1),
                title,
                message,
                page,
            ),
            //TODO: Signature
            // 2 => {}
            _ => Err(ViewError::NoData),
        }
    }
}

#[cfg(test)]
impl<'b> DoubleEndorsementEvidence<'b> {
    pub fn is(&self, _json: &serde_json::Map<std::string::String, serde_json::Value>) {
        //TODO
    }
}

#[cfg(test)]
mod tests {
    use crate::parser::operations::Operation;

    use super::{Endorsement, EndorsementWithSlot};
    use arrayref::array_ref;

    #[test]
    fn endorsement_with_slot() {
        const INPUT_HEX: &str = "00000027\
                                 a99b946c97ada0f42c1bdeae0383db7893351232a832d00d0cd716eb6f66e561\
                                 00\
                                 fffffed4\
                                 0001\
                                 007b";
        const BRANCH_BASE58: &str = "BLzyjjHKEKMULtvkpSHxuZxx6ei6fpntH2BTkYZiLgs8zLVstvX";

        let input = hex::decode(INPUT_HEX).expect("invalid input hex");

        let (rem, parsed) =
            EndorsementWithSlot::from_bytes(&input).expect("failed to parse endorsement");
        assert_eq!(rem.len(), 0);

        let branch =
            Operation::base58_branch(parsed.branch).expect("couldn't encode branch to base58");
        assert_eq!(&branch[..], BRANCH_BASE58.as_bytes());

        let expected = EndorsementWithSlot {
            branch: array_ref!(input, 4, 32),
            endorsement: Endorsement { level: -300 },
            signature: &input[4 + 32 + 1 + 4..4 + 32 + 1 + 4 + 2],
            slot: 123,
        };
        assert_eq!(parsed, expected);
    }

    #[test]
    fn endorsement() {
        const INPUT_HEX: &str = "fffffed4";

        let input = hex::decode(INPUT_HEX).expect("invalid input hex");

        let (rem, parsed) = Endorsement::from_bytes(&input).expect("failed to parse endorsement");
        assert_eq!(rem.len(), 0);

        let expected = Endorsement { level: -300 };
        assert_eq!(parsed, expected);
    }

    #[test]
    fn double_endorsement_evidence() {
        //TODO
    }
}
