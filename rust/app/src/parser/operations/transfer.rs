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
    call, cond, do_parse,
    number::complete::{be_u32, le_u8},
    take, IResult,
};
use zemu_sys::ViewError;

use crate::{
    crypto::Curve,
    handlers::{handle_ui_message, parser_common::ParserError, public_key::Addr},
    parser::{boolean, public_key_hash, DisplayableOperation, Zarith},
};

use super::ContractID;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Entrypoint<'b> {
    Default,
    Root,
    Do,
    SetDelegate,
    RemoveDelegate,
    Custom(&'b [u8]),
}

impl<'b> Entrypoint<'b> {
    pub fn from_bytes(input: &'b [u8]) -> IResult<&[u8], Self, ParserError> {
        let (mut rem, tag) = le_u8(input)?;

        let data = match tag {
            0x00 => Self::Default,
            0x01 => Self::Root,
            0x02 => Self::Do,
            0x03 => Self::SetDelegate,
            0x04 => Self::RemoveDelegate,
            0xFF => {
                let (rem2, length) = le_u8(rem)?;
                let (rem2, name) = take!(rem2, length)?;
                rem = rem2;

                Self::Custom(name)
            }
            _ => return Err(ParserError::parser_invalid_contract_name.into()),
        };

        Ok((rem, data))
    }
}

impl<'b> core::fmt::Display for Entrypoint<'b> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Entrypoint::Default => write!(f, "default"),
            Entrypoint::Root => write!(f, "root"),
            Entrypoint::Do => write!(f, "do"),
            Entrypoint::SetDelegate => write!(f, "set_delegate"),
            Entrypoint::RemoveDelegate => write!(f, "remove_delegate"),
            Entrypoint::Custom(custom) => {
                let custom = core::str::from_utf8(custom).expect("custom entrypoint was not utf8");
                f.write_str(custom)
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, property::Property)]
#[property(mut(disable), get(public), set(disable))]
pub struct Parameters<'b> {
    entrypoint: Entrypoint<'b>,
    michelson: &'b [u8],
}

impl<'b> Parameters<'b> {
    pub fn from_bytes(input: &'b [u8]) -> IResult<&[u8], Self, ParserError> {
        let (rem, (entrypoint, michelson)) = do_parse!(
            input,
            entrypoint: call!(Entrypoint::from_bytes)
                >> length: be_u32
                >> out: take!(length)
                >> (entrypoint, out)
        )?;

        Ok((
            rem,
            Self {
                entrypoint,
                michelson,
            },
        ))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, property::Property)]
#[property(mut(disable), get(public), set(disable))]
pub struct Transfer<'b> {
    source: (Curve, &'b [u8; 20]),
    fee: Zarith<'b>,
    counter: Zarith<'b>,
    gas_limit: Zarith<'b>,
    storage_limit: Zarith<'b>,
    amount: Zarith<'b>,
    destination: ContractID<'b>,
    parameters: Option<Parameters<'b>>,
}

impl<'b> Transfer<'b> {
    #[cfg(not(test))]
    pub fn from_bytes(input: &'b [u8]) -> IResult<&[u8], Self, ParserError> {
        let (
            rem,
            (source, fee, counter, gas_limit, storage_limit, amount, destination, parameters),
        ) = do_parse! {input,
            source: public_key_hash >>
            fee: call!(Zarith::from_bytes, false) >>
            counter: call!(Zarith::from_bytes, false) >>
            gas_limit: call!(Zarith::from_bytes, false) >>
            storage_limit: call!(Zarith::from_bytes, false) >>
            amount: call!(Zarith::from_bytes, false) >>
            destination: call!(ContractID::from_bytes) >>
            has_params: boolean >>
            params: cond!(has_params, Parameters::from_bytes) >>
            (source, fee, counter, gas_limit, storage_limit, amount, destination, params)
        }?;

        Ok((
            rem,
            Self {
                source,
                fee,
                counter,
                gas_limit,
                storage_limit,
                amount,
                destination,
                parameters,
            },
        ))
    }

    #[cfg(test)]
    pub fn from_bytes(input: &'b [u8]) -> IResult<&[u8], Self, ParserError> {
        use nom::dbg_basic;
        use std::println;

        let (
            rem,
            (source, fee, counter, gas_limit, storage_limit, amount, destination, parameters),
        ) = dbg_basic! {input,
            do_parse!(
            source: public_key_hash >>
            fee: call!(Zarith::from_bytes, false) >>
            counter: call!(Zarith::from_bytes, false) >>
            gas_limit: call!(Zarith::from_bytes, false) >>
            storage_limit: call!(Zarith::from_bytes, false) >>
            amount: call!(Zarith::from_bytes, false) >>
            destination: call!(ContractID::from_bytes) >>
            has_params: boolean >>
            params: cond!(has_params, Parameters::from_bytes) >>
            (source, fee, counter, gas_limit, storage_limit, amount, destination, params)
        )}?;

        Ok((
            rem,
            Self {
                source,
                fee,
                counter,
                gas_limit,
                storage_limit,
                amount,
                destination,
                parameters,
            },
        ))
    }

    fn source_base58(&self) -> Result<[u8; 36], bolos::Error> {
        let source = self.source();
        let addr = Addr::from_hash(source.1, source.0)?;

        Ok(addr.to_base58())
    }
}

impl<'a> DisplayableOperation for Transfer<'a> {
    fn num_items(&self) -> usize {
        //TODO: account for entrypoint if present
        1 + 8
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
            //home
            0 => {
                let title_content = pic_str!(b"Type");
                title[..title_content.len()].copy_from_slice(title_content);

                let mex = if self.parameters.is_some() {
                    pic_str!("Contract Execution")
                } else {
                    pic_str!("Transaction")
                };

                handle_ui_message(mex.as_bytes(), message, page)
            }
            //source
            1 => {
                let title_content = pic_str!(b"Source");
                title[..title_content.len()].copy_from_slice(title_content);

                let mex = self.source_base58().map_err(|_| ViewError::Unknown)?;
                handle_ui_message(&mex[..], message, page)
            }
            //destination
            2 => {
                let title_content = if self.destination.is_implicit() {
                    pic_str!("Destination")
                } else {
                    pic_str!("Contract Addr")
                };
                title[..title_content.len()].copy_from_slice(title_content.as_bytes());

                let mut cid = [0; 36];
                self.destination()
                    .base58(&mut cid)
                    .map_err(|_| ViewError::Unknown)?;

                handle_ui_message(&cid[..], message, page)
            }
            //amount
            3 => {
                let title_content = pic_str!(b"Amount");
                title[..title_content.len()].copy_from_slice(title_content);

                let (_, amount) = self.amount().read_as::<usize>().ok_or(ViewError::Unknown)?;

                handle_ui_message(itoa(amount, &mut zarith_buf), message, page)
            }
            //fee
            4 => {
                let title_content = pic_str!(b"Fee");
                title[..title_content.len()].copy_from_slice(title_content);

                let (_, fee) = self.fee().read_as::<usize>().ok_or(ViewError::Unknown)?;

                handle_ui_message(itoa(fee, &mut zarith_buf), message, page)
            }
            //has_parameters
            5 => {
                let title_content = pic_str!(b"Parameters");
                title[..title_content.len()].copy_from_slice(title_content);

                match self.parameters {
                    Some(params) => {
                        use bolos::hash::{Hasher, Sha256};

                        //TODO: display entrypoint

                        //Display sha256 of michelson code
                        let sha =
                            Sha256::digest(params.michelson).map_err(|_| ViewError::Unknown)?;
                        let mut hex_buf = [0; 32 * 2];
                        //this is impossible that will error since the sizes are all checked
                        hex::encode_to_slice(&sha[..], &mut hex_buf).unwrap();

                        handle_ui_message(&hex_buf[..], message, page)
                    }
                    None => handle_ui_message(&pic_str!(b"no parameters...")[..], message, page),
                }
            }
            //gas_limit
            6 => {
                let title_content = pic_str!(b"Gas Limit");
                title[..title_content.len()].copy_from_slice(title_content);

                let (_, gas_limit) = self
                    .gas_limit()
                    .read_as::<usize>()
                    .ok_or(ViewError::Unknown)?;

                handle_ui_message(itoa(gas_limit, &mut zarith_buf), message, page)
            }
            //storage_limit
            7 => {
                let title_content = pic_str!(b"Storage Limit");
                title[..title_content.len()].copy_from_slice(title_content);

                let (_, storage_limit) = self
                    .storage_limit()
                    .read_as::<usize>()
                    .ok_or(ViewError::Unknown)?;

                handle_ui_message(itoa(storage_limit, &mut zarith_buf), message, page)
            }
            //counter
            8 => {
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

#[cfg(test)]
impl<'b> Transfer<'b> {
    pub fn is(&self, json: &serde_json::Map<std::string::String, serde_json::Value>) {
        //verify source address of the transfer
        let source_base58 = self
            .source_base58()
            .expect("couldn't compute source base58");
        let expected_source_base58 = json["source"]
            .as_str()
            .expect("given json .source is not a string");
        assert_eq!(source_base58, expected_source_base58.as_bytes());

        self.amount().is(&json["amount"]);
        self.counter().is(&json["counter"]);
        self.fee().is(&json["fee"]);
        self.gas_limit().is(&json["gas_limit"]);
        self.storage_limit().is(&json["storage_limit"]);

        //verify the destination
        let destination_bs58 = {
            let mut out = [0; 36];
            self.destination()
                .base58(&mut out)
                .expect("couldn't compute destination base58");
            out
        };
        let expected_destination_base58 = json["destination"]
            .as_str()
            .expect("given json .destination is not a string");
        assert_eq!(destination_bs58, expected_destination_base58.as_bytes());

        //check parameters, either they are both in json and the parsed,
        // or they are missing in both
        match (
            self.parameters(),
            json.get("parameters").map(|j| {
                j.as_object()
                    .expect("given json .parameters is not an object")
            }),
        ) {
            (None, None) => {}
            (Some(_), None) => panic!("parsed parameters where none were given"),
            (None, Some(maybe_default)) => {
                let maybe_default = serde_json::Value::Object(maybe_default.clone());
                let default = serde_json::json!({
                    "entrypoint": "default",
                    "value" : {
                        "prim": "Unit"
                    }
                });

                //if it's a `default` then the blob doesn't actually contain anything
                // for the paremeters, so it's ok
                if default != maybe_default {
                    panic!("parameters were not parsed where some were given")
                }
            }
            (Some(parsed), Some(expected)) => {
                //if they are present, verify the entrypoint
                // get entrypoint from json as string
                let expected_entrypoint = expected["entrypoint"]
                    .as_str()
                    .expect("given json .parameters.entrypoint is not a string");

                //verify entrypoint
                match (parsed.entrypoint(), expected_entrypoint) {
                    (Entrypoint::Default, "default")
                    | (Entrypoint::Root, "root")
                    | (Entrypoint::Do, "do")
                    | (Entrypoint::SetDelegate, "set_delegate")
                    | (Entrypoint::RemoveDelegate, "remove_delegate") => {}
                    (Entrypoint::Custom(s), js) if s == &js.as_bytes() => {}
                    (parsed, expected) => {
                        panic!("expected entrypoint: {}, parsed: {}", expected, parsed)
                    }
                }

                //TODO: verify michelson code (parameters.value)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{crypto::Curve, parser::Zarith};

    use super::{ContractID, Entrypoint, Parameters, Transfer};

    #[test]
    fn entrypoint() {
        let (_, default) =
            Entrypoint::from_bytes(&[0]).expect("failed to parse default entrypoint");
        assert_eq!(default, Entrypoint::Default);

        let (_, root) = Entrypoint::from_bytes(&[1]).expect("failed to parse root entrypoint");
        assert_eq!(root, Entrypoint::Root);

        let (_, do_entrypoint) =
            Entrypoint::from_bytes(&[2]).expect("failed to parse do entrypoint");
        assert_eq!(do_entrypoint, Entrypoint::Do);

        let (_, set_delegate) =
            Entrypoint::from_bytes(&[3]).expect("failed to parse set_delegate entrypoint");
        assert_eq!(set_delegate, Entrypoint::SetDelegate);

        let (_, remove_delegate) =
            Entrypoint::from_bytes(&[4]).expect("failed to parse remove_delegate entrypoint");
        assert_eq!(remove_delegate, Entrypoint::RemoveDelegate);

        let (rem, custom) = Entrypoint::from_bytes(&[0xFF, 0x03, 0x61, 0x62, 0x63, 0xaa])
            .expect("failed to parse custom entrypoint");
        assert_eq!(rem.len(), 1);
        assert_eq!(custom, Entrypoint::Custom(b"abc"));
    }

    #[test]
    #[should_panic(expected = "Incomplete(Size(8))")]
    fn entrypoint_eof() {
        Entrypoint::from_bytes(&[0xFF, 10, 0x61, 0x62]).expect("failed to parse custom entrypoint");
    }

    mod parameters {
        use super::{super::Parameters, Entrypoint};

        #[test]
        fn manual() {
            const MICHELSON_CODE: &[u8] = &[0xab, 0xcd];

            let mut input = std::vec![0];
            input.extend_from_slice(&(MICHELSON_CODE.len() as u32).to_be_bytes()[..]);
            input.extend_from_slice(MICHELSON_CODE);
            input.push(0xAA); //dummy byte to verify length

            let (rem, parameters) =
                Parameters::from_bytes(&input).expect("faled to parse parameters");

            assert_eq!(rem.len(), 1);
            assert_eq!(
                parameters,
                Parameters {
                    entrypoint: Entrypoint::Default,
                    michelson: &[0xab, 0xcd]
                }
            )
        }

        #[test]
        fn simple() {
            const INPUT_HEX: &str = "02000000070a000000020202";

            let input = hex::decode(INPUT_HEX).expect("invalid hex input");

            let (_, parameters) =
                Parameters::from_bytes(&input).expect("faled to parse parameters");

            assert_eq!(
                parameters,
                Parameters {
                    entrypoint: Entrypoint::Do,
                    michelson: &input[5..]
                }
            )
        }

        #[test]
        #[should_panic(expected = "Incomplete(Size(8))")]
        fn manual_eof() {
            const MICHELSON_CODE: &[u8] = &[0xab, 0xcd];

            let mut input = std::vec![0];
            input.extend_from_slice(&(10u32).to_be_bytes()[..]);
            input.extend_from_slice(MICHELSON_CODE);

            Parameters::from_bytes(&input).expect("failed to parse parameters");
        }
    }

    #[test]
    fn transfer() {
        const INPUT_HEX: &str = "0035e993d8c7aaa42b5e3ccd86a33390ececc73abd\
                                 904e\
                                 01\
                                 0a\
                                 0a\
                                 e807\
                                 000035e993d8c7aaa42b5e3ccd86a33390ececc73abd\
                                 00";

        let mut input = hex::decode(INPUT_HEX).expect("invalid input hex");
        input.extend_from_slice(&[0xDE, 0xEA, 0xBE, 0xEF]);

        let (rem, parsed) = Transfer::from_bytes(&input).expect("couldn't parse transfer");
        assert_eq!(rem.len(), 4);

        let expected = Transfer {
            source: (Curve::Bip32Ed25519, arrayref::array_ref!(input, 1, 20)),
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
            amount: Zarith {
                is_negative: None,
                bytes: &input[26..28],
            },
            destination: ContractID::Implicit(
                Curve::Bip32Ed25519,
                arrayref::array_ref!(input, 28 + 2, 20),
            ),
            parameters: None,
        };

        assert_eq!(parsed, expected);
    }

    #[test]
    #[should_panic(expected = "Incomplete")]
    fn transfer_eof() {
        const INPUT_HEX: &str = "0035e993d8c7aaa42b5e3ccd86a33390ececc73abd904e01";

        let mut input = hex::decode(INPUT_HEX).expect("invalid input hex");
        input.extend_from_slice(&[0xDE, 0xEA, 0xBE, 0xEF]);

        let (rem, _parsed) = Transfer::from_bytes(&input).expect("couldn't parse transfer");
        assert_eq!(rem.len(), 4);
    }

    #[test]
    fn contract_call() {
        const INPUT_HEX: &str = "0035e993d8c7aaa42b5e3ccd86a33390ececc73abd\
                                 904e\
                                 01\
                                 0a\
                                 0a\
                                 e807\
                                 000035e993d8c7aaa42b5e3ccd86a33390ececc73abd\
                                 ff\
                                 02000000070a000000020202";

        let input = hex::decode(INPUT_HEX).expect("invalid input hex");

        let (rem, parsed) = Transfer::from_bytes(&input).expect("couldn't parse transfer");
        assert_eq!(rem.len(), 0);

        let expected = Transfer {
            source: (Curve::Bip32Ed25519, arrayref::array_ref!(input, 1, 20)),
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
            amount: Zarith {
                is_negative: None,
                bytes: &input[26..28],
            },
            destination: ContractID::Implicit(
                Curve::Bip32Ed25519,
                arrayref::array_ref!(input, 28 + 2, 20),
            ),
            parameters: Some(Parameters {
                entrypoint: Entrypoint::Do,
                michelson: &input[56..],
            }),
        };

        assert_eq!(parsed, expected);
    }
}
