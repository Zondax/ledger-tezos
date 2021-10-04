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
use nom::{number::complete::be_i32, IResult};
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

#[cfg(test)]
mod tests {
    use super::Endorsement;

    #[test]
    fn endorsement() {
        const INPUT_HEX: &str = "fffffed4";

        let input = hex::decode(INPUT_HEX).expect("invalid input hex");

        let (rem, parsed) = Endorsement::from_bytes(&input).expect("failed to parse endorsement");
        assert_eq!(rem.len(), 0);

        let expected = Endorsement { level: -300 };
        assert_eq!(parsed, expected);
    }
}
