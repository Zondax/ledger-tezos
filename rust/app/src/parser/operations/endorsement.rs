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
use core::{mem::MaybeUninit, ptr::addr_of_mut};
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

#[derive(Clone, Copy, PartialEq, Eq, property::Property)]
#[cfg_attr(test, derive(Debug))]
#[property(mut(disable), get(public), set(disable))]
pub struct Endorsement {
    level: i32,
}

impl Endorsement {
    #[inline(never)]
    pub fn from_bytes(input: &[u8]) -> IResult<&[u8], Self, ParserError> {
        let (rem, level) = be_i32(input)?;

        Ok((rem, Self { level }))
    }

    #[inline(never)]
    pub fn from_bytes_into<'b>(
        input: &'b [u8],
        out: &mut core::mem::MaybeUninit<Self>,
    ) -> Result<&'b [u8], nom::Err<ParserError>> {
        let (rem, level) = be_i32(input)?;

        let out = out.as_mut_ptr();
        unsafe {
            addr_of_mut!((*out).level).write(level);
        }

        Ok(rem)
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

#[derive(Clone, Copy, PartialEq, Eq, property::Property)]
#[cfg_attr(test, derive(Debug))]
#[property(mut(disable), get(public), set(disable))]
pub struct EndorsementWithSlot<'b> {
    branch: &'b [u8; 32],
    endorsement: Endorsement,
    signature: &'b [u8],
    slot: u16,
}

impl<'b> EndorsementWithSlot<'b> {
    #[inline(never)]
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

    #[inline(never)]
    pub fn from_bytes_into(
        input: &'b [u8],
        out: &mut MaybeUninit<Self>,
    ) -> Result<&'b [u8], nom::Err<ParserError>> {
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
        let length = (length as usize).checked_sub(32 + 1);
        let rem_len = rem.len() - rem2.len();
        let length = length
            .and_then(|len| len.checked_sub(rem_len))
            .ok_or(ParserError::parser_value_out_of_range)?;
        let (rem, sig) = take(length)(rem2)?;
        let (rem, slot) = be_u16(rem)?;

        let out = out.as_mut_ptr();
        //pointer is valid and we are only writing
        unsafe {
            addr_of_mut!((*out).branch).write(branch);
            addr_of_mut!((*out).endorsement).write(endorsement);
            addr_of_mut!((*out).signature).write(sig);
            addr_of_mut!((*out).slot).write(slot);
        }

        Ok(rem)
    }
}

impl<'b> DisplayableItem for EndorsementWithSlot<'b> {
    fn num_items(&self) -> usize {
        1 + 2 + self.endorsement.num_items()
        //signature
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

                let (len, branch) =
                    super::Operation::base58_branch(self.branch).map_err(|_| ViewError::Unknown)?;

                handle_ui_message(&branch[..len], message, page)
            }
            //Slot
            2 => {
                let title_content = pic_str!(b"Slot");
                title[..title_content.len()].copy_from_slice(title_content);

                let mut mex = [0; u16::FORMATTED_SIZE_DECIMAL];

                handle_ui_message(itoa(self.slot, &mut mex), message, page)
            }
            //Signature
            // 3 => {}
            n => self.endorsement.render_item(n - 3, title, message, page),
        }
    }
}

#[cfg(test)]
impl<'b> EndorsementWithSlot<'b> {
    pub fn is(&self, json: &serde_json::Map<std::string::String, serde_json::Value>) {
        let (len, branch_base58) =
            super::Operation::base58_branch(self.branch).expect("couldn't compute branch base58");
        let expected_branch_base58 = json["branch"]
            .as_str()
            .expect("given json .branch is not a string");
        assert_eq!(&branch_base58[..len], expected_branch_base58.as_bytes());

        let expected = json["slot"]
            .as_i64()
            .expect("given json .slot is not a signed integer");

        assert_eq!(self.slot, expected as u16);

        self.endorsement.is(json)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, property::Property)]
#[cfg_attr(test, derive(Debug))]
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
        let length = (length as usize).checked_sub(32 + 1);
        let rem_len = rem.len() - rem2.len();
        let length = length
            .and_then(|len| len.checked_sub(rem_len))
            .ok_or(ParserError::parser_value_out_of_range)?;
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
        let length = (length as usize).checked_sub(32 + 1);
        let rem_len = rem.len() - rem2.len();
        let length = length
            .and_then(|len| len.checked_sub(rem_len))
            .ok_or(ParserError::parser_value_out_of_range)?;
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

#[cfg(test)]
impl<'b> DoubleEndorsementEvidence<'b> {
    pub fn is(&self, _json: &serde_json::Map<std::string::String, serde_json::Value>) {}
}

#[cfg(test)]
mod tests {
    use crate::parser::operations::Operation;

    use super::{Endorsement, EndorsementWithSlot};
    use arrayref::array_ref;

    #[test]
    fn endorsement_with_slot() {
        //Note: this is madeup input based on the codec description
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

        let (len, branch) =
            Operation::base58_branch(parsed.branch).expect("couldn't encode branch to base58");
        assert_eq!(&branch[..len], BRANCH_BASE58.as_bytes());

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
    fn double_endorsement_evidence() {}
}
