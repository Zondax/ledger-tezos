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
use nom::{bytes::complete::take, number::complete::be_i32, IResult};
use zemu_sys::ViewError;

use crate::{
    handlers::{handle_ui_message, parser_common::ParserError},
    parser::DisplayableItem,
};

const SEED_NONCE_BYTES_LEN: usize = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq, property::Property)]
#[property(mut(disable), get(public), set(disable))]
pub struct SeedNonceRevelation<'b> {
    level: i32,
    nonce: &'b [u8; SEED_NONCE_BYTES_LEN],
}

impl<'b> SeedNonceRevelation<'b> {
    pub fn from_bytes(input: &'b [u8]) -> IResult<&[u8], Self, ParserError> {
        let (rem, level) = be_i32(input)?;
        let (rem, bytes) = take(SEED_NONCE_BYTES_LEN)(rem)?;
        let nonce = arrayref::array_ref!(bytes, 0, SEED_NONCE_BYTES_LEN);

        Ok((rem, Self { level, nonce }))
    }
}

impl<'b> DisplayableItem for SeedNonceRevelation<'b> {
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
        use bolos::{pic_str, PIC};
        use lexical_core::{write as itoa, Number};

        match item_n {
            //Homepage
            0 => {
                let title_content = pic_str!(b"Type");
                title[..title_content.len()].copy_from_slice(title_content);

                handle_ui_message(&pic_str!(b"Seed Nonce Revelation")[..], message, page)
            }
            //Level
            1 => {
                let title_content = pic_str!(b"Level");
                title[..title_content.len()].copy_from_slice(title_content);

                let mut itoa_buf = [0; i32::FORMATTED_SIZE_DECIMAL];

                handle_ui_message(itoa(self.level, &mut itoa_buf), message, page)
            }
            //Nonce
            2 => {
                let title_content = pic_str!(b"Nonce");
                title[..title_content.len()].copy_from_slice(title_content);

                let mut hex_buf = [0; SEED_NONCE_BYTES_LEN * 2];
                //this is impossible that will error since the sizes are all checked
                hex::encode_to_slice(self.nonce, &mut hex_buf).unwrap();

                handle_ui_message(&hex_buf[..], message, page)
            }
            _ => Err(ViewError::NoData),
        }
    }
}

#[cfg(test)]
impl<'b> SeedNonceRevelation<'b> {
    pub fn is(&self, json: &serde_json::Map<std::string::String, serde_json::Value>) {
        let expected_level = json["level"]
            .as_i64()
            .expect("given json .level is not a signed integer");

        assert_eq!(self.level, expected_level as i32);

        let expected_nonce = json["nonce"]
            .as_str()
            .expect("given json .nonce is not a string");
        let expected_nonce =
            hex::decode(expected_nonce).expect("given json .nonce is not a hex string");

        assert_eq!(self.nonce, &expected_nonce[..]);
    }
}

#[cfg(test)]
mod tests {
    use super::SeedNonceRevelation;

    #[test]
    fn seed_nonce_revelation() {
        const INPUT_HEX: &str = "000063ce\
                                 ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff";

        let input = hex::decode(INPUT_HEX).expect("invalid input hex");

        let (rem, parsed) =
            SeedNonceRevelation::from_bytes(&input).expect("failed to parse endorsement");
        assert_eq!(rem.len(), 0);

        let expected = SeedNonceRevelation {
            level: 25550,
            nonce: &[0xFF; 32],
        };
        assert_eq!(parsed, expected);
    }
}
