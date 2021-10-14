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
use core::{mem::MaybeUninit, ptr::addr_of_mut};
use nom::{call, cond, do_parse, IResult};
use zemu_sys::ViewError;

use crate::{
    crypto::Curve,
    handlers::{handle_ui_message, parser_common::ParserError, public_key::Addr},
    parser::{boolean, public_key_hash, DisplayableItem, Zarith},
};

#[derive(Debug, Clone, Copy, PartialEq, property::Property)]
#[property(mut(disable), get(public), set(disable))]
pub struct Delegation<'b> {
    source: (Curve, &'b [u8; 20]),
    fee: Zarith<'b>,
    counter: Zarith<'b>,
    gas_limit: Zarith<'b>,
    storage_limit: Zarith<'b>,
    delegate: Option<(Curve, &'b [u8; 20])>,
}

impl<'b> Delegation<'b> {
    #[inline(never)]
    pub fn from_bytes(input: &'b [u8]) -> IResult<&[u8], Self, ParserError> {
        crate::sys::zemu_log_stack("Delegation::from_bytes\x00");

        let (rem, (source, fee, counter, gas_limit, storage_limit, delegate)) = do_parse! {input,
            source: public_key_hash >>
            fee: call!(Zarith::from_bytes, false) >>
            counter: call!(Zarith::from_bytes, false) >>
            gas_limit: call!(Zarith::from_bytes, false) >>
            storage_limit: call!(Zarith::from_bytes, false) >>
            has_delegate: boolean >>
            delegate: cond!(has_delegate, public_key_hash) >>
            (source, fee, counter, gas_limit, storage_limit, delegate)
        }?;

        Ok((
            rem,
            Self {
                source,
                fee,
                counter,
                gas_limit,
                storage_limit,
                delegate,
            },
        ))
    }

    #[inline(never)]
    pub fn from_bytes_into(
        input: &'b [u8],
        out: &mut MaybeUninit<Self>,
    ) -> Result<&'b [u8], nom::Err<ParserError>> {
        crate::sys::zemu_log_stack("Delegation::from_bytes\x00");

        let (rem, (source, fee, counter, gas_limit, storage_limit, delegate)) = do_parse! {input,
            source: public_key_hash >>
            fee: call!(Zarith::from_bytes, false) >>
            counter: call!(Zarith::from_bytes, false) >>
            gas_limit: call!(Zarith::from_bytes, false) >>
            storage_limit: call!(Zarith::from_bytes, false) >>
            has_delegate: boolean >>
            delegate: cond!(has_delegate, public_key_hash) >>
            (source, fee, counter, gas_limit, storage_limit, delegate)
        }?;

        let out = out.as_mut_ptr();
        //good ptr and no uninit reads
        unsafe {
            addr_of_mut!((*out).source).write(source);
            addr_of_mut!((*out).fee).write(fee);
            addr_of_mut!((*out).counter).write(counter);
            addr_of_mut!((*out).gas_limit).write(gas_limit);
            addr_of_mut!((*out).storage_limit).write(storage_limit);
            addr_of_mut!((*out).delegate).write(delegate);
        }

        Ok(rem)
    }
}

impl<'a> DisplayableItem for Delegation<'a> {
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
            //home
            0 => {
                let title_content = pic_str!(b"Type");
                title[..title_content.len()].copy_from_slice(title_content);

                let mex = if self.delegate.is_some() {
                    pic_str!("Delegation")
                } else {
                    pic_str!("Delegation Withdrawal")
                };

                handle_ui_message(mex.as_bytes(), message, page)
            }
            //source
            1 => {
                let title_content = pic_str!(b"Source");
                title[..title_content.len()].copy_from_slice(title_content);

                let (crv, hash) = self.source();

                let addr = Addr::from_hash(hash, *crv).map_err(|_| ViewError::Unknown)?;

                let (len, mex) = addr.base58();
                handle_ui_message(&mex[..len], message, page)
            }
            //delegation
            2 => {
                let title_content = pic_str!(b"Delegation");
                title[..title_content.len()].copy_from_slice(title_content);

                match self.delegate {
                    Some((crv, hash)) => {
                        match baker_lookup(arrayref::array_ref!(crv.to_hash_prefix(), 0, 3), &hash)
                        {
                            Ok(name) => handle_ui_message(name.as_bytes(), message, page),
                            Err(_) => {
                                let addr =
                                    Addr::from_hash(hash, crv).map_err(|_| ViewError::Unknown)?;
                                let (len, mex) = addr.base58();
                                handle_ui_message(&mex[..len], message, page)
                            }
                        }
                    }
                    None => handle_ui_message(&pic_str!(b"<REVOKED>")[..], message, page),
                }
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

#[cfg(test)]
impl<'b> Delegation<'b> {
    fn addr_base58(
        &self,
        source: (Curve, &'b [u8; 20]),
    ) -> Result<(usize, [u8; Addr::BASE58_LEN]), bolos::Error> {
        let addr = Addr::from_hash(source.1, source.0)?;

        Ok(addr.base58())
    }

    pub fn is(&self, json: &serde_json::Map<std::string::String, serde_json::Value>) {
        //verify source address of the transfer
        let (len, source_base58) = self
            .addr_base58(*self.source())
            .expect("couldn't compute source base58");
        let expected_source_base58 = json["source"]
            .as_str()
            .expect("given json .source is not a string");
        assert_eq!(&source_base58[..len], expected_source_base58.as_bytes());

        self.counter().is(&json["counter"]);
        self.fee().is(&json["fee"]);
        self.gas_limit().is(&json["gas_limit"]);
        self.storage_limit().is(&json["storage_limit"]);

        match (
            self.delegate(),
            json.get("delegate")
                .map(|j| j.as_str().expect("given json .delegate is not a string")),
        ) {
            (None, None) => {}
            (Some(_), None) => panic!("parsed delegate where none were given"),
            (None, Some(_)) => panic!("delegate was not parsed where one was given"),
            (Some(parsed), Some(expected_delegate_base58)) => {
                let (len, delegate_base58) = self
                    .addr_base58(*parsed)
                    .expect("couldn't compute delegate base58");
                assert_eq!(&delegate_base58[..len], expected_delegate_base58.as_bytes())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        crypto::Curve,
        parser::{operations::Delegation, Zarith},
    };

    #[test]
    fn delegation() {
        const INPUT_HEX: &str = "0035e993d8c7aaa42b5e3ccd86a33390ececc73abd\
                                 904e\
                                 01\
                                 0a\
                                 0a\
                                 ff\
                                 0035e993d8c7aaa42b5e3ccd86a33390ececc73abd";

        let mut input = hex::decode(INPUT_HEX).expect("invalid input hex");
        input.extend_from_slice(&[0xDE, 0xEA, 0xBE, 0xEF]);

        let (rem, parsed) = Delegation::from_bytes(&input).expect("couldn't parse delegation");
        assert_eq!(rem.len(), 4);

        let expected = Delegation {
            //0 is the 00 to identify implicit contract
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
            //27 is bool
            //28 is the 00 to identify implicit contract
            delegate: Some((Curve::Bip32Ed25519, arrayref::array_ref!(input, 28, 20))),
        };

        assert_eq!(parsed, expected);
    }
}

mod known_bakers {
    use bolos::PIC;
    use zemu_sys::zemu_log_stack;

    ledger_tezos_derive::unroll!("vendor/BakersRegistryCoreUnfilteredData.json");
}
use known_bakers::baker_lookup;
