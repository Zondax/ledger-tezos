use arrayref::array_ref;
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
use nom::{do_parse, take, IResult};
use zemu_sys::ViewError;

use crate::{
    crypto::Curve,
    handlers::{handle_ui_message, parser_common::ParserError, public_key::Addr},
    parser::DisplayableItem,
};

#[derive(Debug, Clone, Copy, PartialEq, property::Property)]
#[property(mut(disable), get(public), set(disable))]
pub struct ActivateAccount<'b> {
    public_key_hash: (Curve, &'b [u8; 20]),
    secret: &'b [u8; 20],
}

impl<'b> ActivateAccount<'b> {
    pub fn from_bytes(input: &'b [u8]) -> IResult<&[u8], Self, ParserError> {
        let (rem, (public_key_hash, secret)) = do_parse! {input,
            public_key_hash: take!(20) >>
            secret: take!(20) >>
            (public_key_hash, secret)
        }?;

        let public_key_hash = array_ref!(public_key_hash, 0, 20);
        let secret = array_ref!(secret, 0, 20);

        Ok((
            rem,
            Self {
                public_key_hash: (Curve::Bip32Ed25519, public_key_hash), //only Ed25519 keys are allowed here
                secret,
            },
        ))
    }

    fn source_base58(&self) -> Result<[u8; 36], bolos::Error> {
        let source = self.public_key_hash;
        let addr = Addr::from_hash(source.1, source.0)?;

        Ok(addr.base58())
    }
}

impl<'b> DisplayableItem for ActivateAccount<'b> {
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

        match item_n {
            //Homepage
            0 => {
                let title_content = pic_str!(b"Type");
                title[..title_content.len()].copy_from_slice(title_content);

                handle_ui_message(&pic_str!(b"Acct. Activation")[..], message, page)
            }
            //public key hash
            1 => {
                let title_content = pic_str!(b"Public Key Hash");
                title[..title_content.len()].copy_from_slice(title_content);

                let mex = self.source_base58().map_err(|_| ViewError::Unknown)?;
                handle_ui_message(&mex[..], message, page)
            }
            //secret
            2 => {
                let title_content = pic_str!("Secret");
                title[..title_content.len()].copy_from_slice(title_content.as_bytes());

                let mut hex_buf = [0; 20 * 2];
                //this is impossible that will error since the sizes are all checked
                hex::encode_to_slice(self.secret, &mut hex_buf).unwrap();

                handle_ui_message(&hex_buf, message, page)
            }
            _ => Err(ViewError::NoData),
        }
    }
}

#[cfg(test)]
impl<'b> ActivateAccount<'b> {
    pub fn is(&self, json: &serde_json::Map<std::string::String, serde_json::Value>) {
        //verify source address of the transfer
        let source_base58 = self.source_base58().expect("couldn't compute pkh base58");
        let expected_source_base58 = json["pkh"]
            .as_str()
            .expect("given json .pkh is not a string");
        assert_eq!(source_base58, expected_source_base58.as_bytes());

        let expected_secret = json["secret"]
            .as_str()
            .expect("given json .secret is not a string");
        let expected_secret =
            hex::decode(expected_secret).expect("given json .secret is not a hex string");

        assert_eq!(self.secret, &expected_secret[..]);
    }
}

#[cfg(test)]
mod tests {
    use arrayref::array_ref;

    use crate::crypto::Curve;

    use super::ActivateAccount;

    #[test]
    fn activate_account() {
        const INPUT_HEX: &str = "b2e19a9e74440d86c59f13dab8a18ff873e889ea\
                                 7d4c8c3796fdbf4869edb5703758f0e5831f5081";

        let input = hex::decode(INPUT_HEX).expect("invalid input hex");

        let (rem, parsed) =
            ActivateAccount::from_bytes(&input).expect("failed to parse endorsement");
        assert_eq!(rem.len(), 0);

        let expected = ActivateAccount {
            public_key_hash: (Curve::Bip32Ed25519, array_ref!(input, 0, 20)),
            secret: array_ref!(input, 20, 20),
        };
        assert_eq!(parsed, expected);
    }
}
