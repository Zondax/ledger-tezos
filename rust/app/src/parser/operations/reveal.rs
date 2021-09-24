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
use nom::{call, do_parse, IResult};
use zemu_sys::ViewError;

use crate::{
    crypto::Curve,
    handlers::{handle_ui_message, parser_common::ParserError, public_key::Addr, sha256x2},
    parser::{public_key, public_key_hash, DisplayableItem, Zarith},
};

#[cfg(test)]
use crate::utils::MaybeNullTerminatedToString;

#[derive(Debug, Clone, Copy, PartialEq, property::Property)]
#[property(mut(disable), get(public), set(disable))]
pub struct Reveal<'b> {
    source: (Curve, &'b [u8; 20]),
    fee: Zarith<'b>,
    counter: Zarith<'b>,
    gas_limit: Zarith<'b>,
    storage_limit: Zarith<'b>,
    public_key: (Curve, &'b [u8]),
}

impl<'b> Reveal<'b> {
    pub fn from_bytes(input: &'b [u8]) -> IResult<&[u8], Self, ParserError> {
        let (rem, (source, fee, counter, gas_limit, storage_limit, public_key)) = do_parse! {input,
            source: public_key_hash >>
            fee: call!(Zarith::from_bytes, false) >>
            counter: call!(Zarith::from_bytes, false) >>
            gas_limit: call!(Zarith::from_bytes, false) >>
            storage_limit: call!(Zarith::from_bytes, false) >>
            public_key: public_key >>
            (source, fee, counter, gas_limit, storage_limit, public_key)
        }?;

        Ok((
            rem,
            Self {
                source,
                fee,
                counter,
                gas_limit,
                storage_limit,
                public_key,
            },
        ))
    }

    fn source_base58(&self) -> Result<[u8; Addr::BASE58_LEN], bolos::Error> {
        let source = self.source;
        let addr = Addr::from_hash(source.1, source.0)?;

        Ok(addr.base58())
    }
}

impl<'b> DisplayableItem for Reveal<'b> {
    fn num_items(&self) -> usize {
        1 + 6
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

        let mut zarith_buf = [0; usize::FORMATTED_SIZE_DECIMAL];

        match item_n {
            //Homepage
            0 => {
                let title_content = pic_str!(b"Type");
                title[..title_content.len()].copy_from_slice(title_content);

                handle_ui_message(&pic_str!(b"Revelation")[..], message, page)
            }
            //source
            1 => {
                let title_content = pic_str!(b"Source");
                title[..title_content.len()].copy_from_slice(title_content);

                let mex = self.source_base58().map_err(|_| ViewError::Unknown)?;
                handle_ui_message(&mex[..], message, page)
            }
            //public key
            2 => {
                let title_content = pic_str!("Public Key");
                title[..title_content.len()].copy_from_slice(title_content.as_bytes());

                let mut public_key = [0; MAX_PK_BASE58_LEN];
                let pk_len = pk_to_base58(self.public_key, &mut public_key)
                    .map_err(|_| ViewError::Unknown)?;

                handle_ui_message(&public_key[..pk_len], message, page)
            }
            //fee
            3 => {
                let title_content = pic_str!(b"Fee");
                title[..title_content.len()].copy_from_slice(title_content);

                let (_, fee) = self.fee().read_as::<usize>().ok_or(ViewError::Unknown)?;

                handle_ui_message(itoa(fee, &mut zarith_buf), message, page)
            }
            //gas_limit
            4 => {
                let title_content = pic_str!(b"Gas Limit");
                title[..title_content.len()].copy_from_slice(title_content);

                let (_, gas_limit) = self
                    .gas_limit()
                    .read_as::<usize>()
                    .ok_or(ViewError::Unknown)?;

                handle_ui_message(itoa(gas_limit, &mut zarith_buf), message, page)
            }
            //storage_limit
            5 => {
                let title_content = pic_str!(b"Storage Limit");
                title[..title_content.len()].copy_from_slice(title_content);

                let (_, storage_limit) = self
                    .storage_limit()
                    .read_as::<usize>()
                    .ok_or(ViewError::Unknown)?;

                handle_ui_message(itoa(storage_limit, &mut zarith_buf), message, page)
            }
            //counter
            6 => {
                let title_content = pic_str!(b"Counter");
                title[..title_content.len()].copy_from_slice(title_content);

                let (_, counter) = self
                    .counter()
                    .read_as::<usize>()
                    .ok_or(ViewError::Unknown)?;

                handle_ui_message(itoa(counter, &mut zarith_buf), message, page)
            }
            _ => Err(ViewError::NoData),
        }
    }
}

const MAX_PK_BASE58_LEN: usize = 56;
/// Encodes a public key as base58 on the provided `out` buffer
///
/// returns the number of bytes written
fn pk_to_base58(
    (crv, bytes): (Curve, &[u8]),
    out: &mut [u8; MAX_PK_BASE58_LEN],
) -> Result<usize, bolos::Error> {
    let prefix = crv.to_prefix();

    let mut checksum = [0; 4];
    sha256x2(&[prefix, bytes], &mut checksum)?;

    let (len, input) = {
        //initialize with max len
        let mut array = [0; 4 + 33 + 4];
        array[..4].copy_from_slice(prefix);
        array[4..4 + bytes.len()].copy_from_slice(bytes);
        array[4 + bytes.len()..4 + bytes.len() + 4].copy_from_slice(&checksum[..]);
        (4 + bytes.len() + 4, array)
    };

    let len = bs58::encode(&input[..len])
        .into(&mut out[..])
        .expect("encoded in base58 is not of the right length");
    Ok(len)
}

#[cfg(test)]
impl<'b> Reveal<'b> {
    pub fn is(&self, json: &serde_json::Map<std::string::String, serde_json::Value>) {
        //verify source address of the transfer
        let source_base58 = self
            .source_base58()
            .expect("couldn't compute source base58")
            .to_string_with_check_null()
            .expect("source base58 was not utf-8");
        let expected_source_base58 = json["source"]
            .as_str()
            .expect("given json .source is not a string");
        assert_eq!(source_base58.as_str(), expected_source_base58);

        self.counter.is(&json["counter"]);
        self.fee.is(&json["fee"]);
        self.gas_limit.is(&json["gas_limit"]);
        self.storage_limit.is(&json["storage_limit"]);

        //verify public key
        let mut pk_base58 = [0; MAX_PK_BASE58_LEN];
        let pk_base58_len = pk_to_base58(self.public_key, &mut pk_base58)
            .expect("couldn't compute public key base58");

        let expected_pk_base58 = json["public_key"]
            .as_str()
            .expect("given json .source is not a string");

        assert_eq!(&pk_base58[..pk_base58_len], expected_pk_base58.as_bytes());
    }
}

#[cfg(test)]
mod tests {
    use arrayref::array_ref;

    use crate::{
        crypto::Curve,
        parser::{operations::reveal::MAX_PK_BASE58_LEN, Zarith},
    };

    use super::{pk_to_base58, Reveal};

    #[test]
    fn reveal() {
        const INPUT_HEX: &str = "0035e993d8c7aaa42b5e3ccd86a33390ececc73abd\
                                 904e\
                                 01\
                                 0a\
                                 0a\
                                 00ebcf82872f4942052704e95dc4bfa0538503dbece27414a39b6650bcecbff896";

        let input = hex::decode(INPUT_HEX).expect("invalid input hex");

        let (rem, parsed) = Reveal::from_bytes(&input).expect("failed to parse endorsement");
        assert_eq!(rem.len(), 0);

        let expected = Reveal {
            source: (Curve::Bip32Ed25519, array_ref!(input, 1, 20)),
            fee: Zarith {
                is_negative: None,
                bytes: &input[21..23],
            },
            counter: Zarith {
                is_negative: None,
                bytes: &input[23..24],
            },
            gas_limit: Zarith {
                is_negative: None,
                bytes: &input[24..25],
            },
            storage_limit: Zarith {
                is_negative: None,
                bytes: &input[25..26],
            },
            public_key: (Curve::Bip32Ed25519, array_ref!(input, 26 + 1, 32)),
        };
        assert_eq!(parsed, expected);
    }

    #[test]
    fn public_key_base58() {
        let mut base58 = [0; MAX_PK_BASE58_LEN];

        let len = pk_to_base58((Curve::Bip32Ed25519, &[0x00; 32]), &mut base58)
            .expect("couldn't encode Secp256K1 to base58");
        assert_eq!(len, 54);

        let len = pk_to_base58((Curve::Bip32Ed25519, &[0xff; 32]), &mut base58)
            .expect("couldn't encode Secp256K1 to base58");
        assert_eq!(len, 54);

        let len = pk_to_base58((Curve::Secp256K1, &[0; 33]), &mut base58)
            .expect("couldn't encode Secp256K1 to base58");
        assert_eq!(len, 55);

        let len = pk_to_base58((Curve::Secp256K1, &[0xff; 33]), &mut base58)
            .expect("couldn't encode Secp256K1 to base58");
        assert_eq!(len, 55);

        let len = pk_to_base58((Curve::Secp256R1, &[0; 33]), &mut base58)
            .expect("couldn't encode Secp256K1 to base58");
        assert_eq!(len, 55);

        let len = pk_to_base58((Curve::Secp256R1, &[0xff; 33]), &mut base58)
            .expect("couldn't encode Secp256K1 to base58");
        assert_eq!(len, 55);
    }
}
