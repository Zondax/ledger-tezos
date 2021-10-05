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
use nom::{do_parse, number::complete::be_u32, take, IResult};
use zemu_sys::ViewError;
use core::{mem::MaybeUninit, ptr::addr_of_mut};

use crate::{
    handlers::{handle_ui_message, parser_common::ParserError},
    parser::DisplayableItem,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, property::Property)]
#[property(mut(disable), get(public), set(disable))]
pub struct FailingNoop<'b> {
    arbitrary: &'b [u8],
}

impl<'b> FailingNoop<'b> {
    #[inline(never)]
    pub fn from_bytes(input: &'b [u8]) -> IResult<&[u8], Self, ParserError> {
        let (rem, arbitrary) =
            do_parse!(input, len: be_u32 >> arbitrary: take!(len) >> (arbitrary))?;

        Ok((rem, Self { arbitrary }))
    }

    #[inline(never)]
    pub fn from_bytes_into(
        input: &'b [u8],
        out: &mut MaybeUninit<Self>,
    ) -> Result<&'b [u8], nom::Err<ParserError>> {
        let (rem, arbitrary) =
            do_parse!(input, len: be_u32 >> arbitrary: take!(len) >> (arbitrary))?;

        let out = out.as_mut_ptr();
        //unsafe ptr valid and no uninit data read
        unsafe { addr_of_mut!((*out).arbitrary).write(arbitrary) }

        Ok(rem)
    }
}

impl<'a> DisplayableItem for FailingNoop<'a> {
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
        use bolos::{
            hash::{Hasher, Sha256},
            pic_str, PIC,
        };

        match item_n {
            //home
            0 => {
                let title_content = pic_str!(b"Type");
                title[..title_content.len()].copy_from_slice(title_content);

                let mex = pic_str!("Failing Noop");

                handle_ui_message(mex.as_bytes(), message, page)
            }
            //source
            1 => {
                let title_content = pic_str!(b"Data Hash");
                title[..title_content.len()].copy_from_slice(title_content);

                let sha = Sha256::digest(self.arbitrary).map_err(|_| ViewError::Unknown)?;
                let mut hex_buf = [0; 32 * 2];
                //this is impossible that will error since the sizes are all checked
                hex::encode_to_slice(&sha[..], &mut hex_buf).unwrap();

                handle_ui_message(&hex_buf[..], message, page)
            }
            _ => Err(ViewError::NoData),
        }
    }
}

#[cfg(test)]
impl<'b> FailingNoop<'b> {
    pub fn is(&self, _json: &serde_json::Map<std::string::String, serde_json::Value>) {
        //TODO: verify arbitrary
    }
}

#[cfg(test)]
mod tests {
    use super::FailingNoop;

    #[test]
    fn failing_noop() {
        const INPUT_HEX: &str = "0000005c\
                                 070707070100000024747a31515a364b5937643342755a4454316431396455786f51727446504e32514a33686e030607070100000024747a31515a364b5937643342755a4454316431396455786f51727446504e32514a33686e0306";

        let input = hex::decode(INPUT_HEX).expect("invalid input hex");

        let (rem, parsed) = FailingNoop::from_bytes(&input).expect("failed to parse endorsement");
        assert_eq!(rem.len(), 0);

        let expected = FailingNoop {
            arbitrary: &input[4..],
        };
        assert_eq!(parsed, expected);
    }
}
