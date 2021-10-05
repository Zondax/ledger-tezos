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
use nom::{call, cond, do_parse, number::complete::be_u32, take, IResult};
use zemu_sys::ViewError;

use crate::{
    crypto::Curve,
    handlers::{handle_ui_message, parser_common::ParserError, public_key::Addr},
    parser::{boolean, public_key_hash, DisplayableItem, Zarith},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, property::Property)]
#[property(mut(disable), get(public), set(disable))]
pub struct Script<'b> {
    code: &'b [u8],
    storage: &'b [u8],
}

impl<'b> Script<'b> {
    pub fn from_bytes(input: &'b [u8]) -> IResult<&[u8], Self, ParserError> {
        let (rem, (code, storage)) = do_parse!(
            input,
            code_len: be_u32
                >> code: take!(code_len)
                >> storage_len: be_u32
                >> storage: take!(storage_len)
                >> (code, storage)
        )?;

        Ok((rem, Self { code, storage }))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, property::Property)]
#[property(mut(disable), get(public), set(disable))]
pub struct Origination<'b> {
    source: (Curve, &'b [u8; 20]),
    fee: Zarith<'b>,
    counter: Zarith<'b>,
    gas_limit: Zarith<'b>,
    storage_limit: Zarith<'b>,
    balance: Zarith<'b>,
    delegate: Option<(Curve, &'b [u8; 20])>,
    script: Script<'b>,
}

impl<'b> Origination<'b> {
    #[inline(never)]
    pub fn from_bytes(input: &'b [u8]) -> IResult<&[u8], Self, ParserError> {
        let (rem, (source, fee, counter, gas_limit, storage_limit, balance, delegate, script)) = do_parse! {input,
            source: public_key_hash >>
            fee: call!(Zarith::from_bytes, false) >>
            counter: call!(Zarith::from_bytes, false) >>
            gas_limit: call!(Zarith::from_bytes, false) >>
            storage_limit: call!(Zarith::from_bytes, false) >>
            balance: call!(Zarith::from_bytes, false) >>
            has_delegate: boolean >>
            delegate: cond!(has_delegate, public_key_hash) >>
            script: call!(Script::from_bytes) >>
            (source, fee, counter, gas_limit, storage_limit, balance, delegate, script)
        }?;

        Ok((
            rem,
            Self {
                source,
                fee,
                counter,
                gas_limit,
                storage_limit,
                balance,
                delegate,
                script,
            },
        ))
    }

    #[inline(never)]
    pub fn from_bytes_into(
        input: &'b [u8],
        out: &mut MaybeUninit<Self>,
    ) -> Result<&'b [u8], nom::Err<ParserError>> {
        let (rem, (source, fee, counter, gas_limit, storage_limit, balance, delegate, script)) = do_parse! {input,
            source: public_key_hash >>
            fee: call!(Zarith::from_bytes, false) >>
            counter: call!(Zarith::from_bytes, false) >>
            gas_limit: call!(Zarith::from_bytes, false) >>
            storage_limit: call!(Zarith::from_bytes, false) >>
            balance: call!(Zarith::from_bytes, false) >>
            has_delegate: boolean >>
            delegate: cond!(has_delegate, public_key_hash) >>
            script: call!(Script::from_bytes) >>
            (source, fee, counter, gas_limit, storage_limit, balance, delegate, script)
        }?;

        let out = out.as_mut_ptr();
        //good ptr and no uninit reads
        unsafe {
            addr_of_mut!((*out).source).write(source);
            addr_of_mut!((*out).fee).write(fee);
            addr_of_mut!((*out).counter).write(counter);
            addr_of_mut!((*out).gas_limit).write(gas_limit);
            addr_of_mut!((*out).storage_limit).write(storage_limit);
            addr_of_mut!((*out).balance).write(balance);
            addr_of_mut!((*out).delegate).write(delegate);
            addr_of_mut!((*out).script).write(script);
        }

        Ok(rem)
    }

    fn source_base58(&self) -> Result<(usize, [u8; Addr::BASE58_LEN]), bolos::Error> {
        let source = self.source;
        let addr = Addr::from_hash(source.1, source.0)?;

        Ok(addr.base58())
    }

    fn delegate_base58(&self) -> Result<Option<(usize, [u8; Addr::BASE58_LEN])>, bolos::Error> {
        self.delegate
            .map(|(crv, hash)| Addr::from_hash(hash, crv).map(|a| a.base58()))
            .transpose()
    }
}

impl<'a> DisplayableItem for Origination<'a> {
    fn num_items(&self) -> usize {
        1 + 9
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
        use lexical_core::{write as itoa, Number};

        let mut zarith_buf = [0; usize::FORMATTED_SIZE_DECIMAL];

        match item_n {
            //home
            0 => {
                let title_content = pic_str!(b"Type");
                title[..title_content.len()].copy_from_slice(title_content);

                let mex = pic_str!("Origination");

                handle_ui_message(mex.as_bytes(), message, page)
            }
            //source
            1 => {
                let title_content = pic_str!(b"Source");
                title[..title_content.len()].copy_from_slice(title_content);

                let (len, mex) = self.source_base58().map_err(|_| ViewError::Unknown)?;
                handle_ui_message(&mex[..len], message, page)
            }
            //balance
            2 => {
                let title_content = pic_str!(b"Balance");
                title[..title_content.len()].copy_from_slice(title_content);

                let (_, amount) = self.balance.read_as::<usize>().ok_or(ViewError::Unknown)?;

                handle_ui_message(itoa(amount, &mut zarith_buf), message, page)
            }
            //delegate
            3 => {
                let title_content = pic_str!(b"Delegate");
                title[..title_content.len()].copy_from_slice(title_content);

                match self.delegate_base58().map_err(|_| ViewError::Unknown)? {
                    Some((len, delegate)) => handle_ui_message(&delegate[..len], message, page),
                    None => handle_ui_message(&pic_str!(b"no delegate")[..], message, page),
                }
            }
            //fee
            4 => {
                let title_content = pic_str!(b"Fee");
                title[..title_content.len()].copy_from_slice(title_content);

                let (_, fee) = self.fee().read_as::<usize>().ok_or(ViewError::Unknown)?;

                handle_ui_message(itoa(fee, &mut zarith_buf), message, page)
            }
            //Script code
            5 => {
                let title_content = pic_str!(b"Code");
                title[..title_content.len()].copy_from_slice(title_content);

                let sha = Sha256::digest(self.script.code).map_err(|_| ViewError::Unknown)?;
                let mut hex_buf = [0; 32 * 2];
                //this is impossible that will error since the sizes are all checked
                hex::encode_to_slice(&sha[..], &mut hex_buf).unwrap();

                handle_ui_message(&hex_buf[..], message, page)
            }
            //Script storage
            6 => {
                let title_content = pic_str!(b"Storage");
                title[..title_content.len()].copy_from_slice(title_content);

                let sha = Sha256::digest(self.script.storage).map_err(|_| ViewError::Unknown)?;
                let mut hex_buf = [0; 32 * 2];
                //this is impossible that will error since the sizes are all checked
                hex::encode_to_slice(&sha[..], &mut hex_buf).unwrap();

                handle_ui_message(&hex_buf[..], message, page)
            }
            //gas_limit
            7 => {
                let title_content = pic_str!(b"Gas Limit");
                title[..title_content.len()].copy_from_slice(title_content);

                let (_, gas_limit) = self
                    .gas_limit()
                    .read_as::<usize>()
                    .ok_or(ViewError::Unknown)?;

                handle_ui_message(itoa(gas_limit, &mut zarith_buf), message, page)
            }
            //storage_limit
            8 => {
                let title_content = pic_str!(b"Storage Limit");
                title[..title_content.len()].copy_from_slice(title_content);

                let (_, storage_limit) = self
                    .storage_limit()
                    .read_as::<usize>()
                    .ok_or(ViewError::Unknown)?;

                handle_ui_message(itoa(storage_limit, &mut zarith_buf), message, page)
            }
            //counter
            9 => {
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
impl<'b> Origination<'b> {
    pub fn is(&self, json: &serde_json::Map<std::string::String, serde_json::Value>) {
        //verify source address of the transfer
        let (len, source_base58) = self
            .source_base58()
            .expect("couldn't compute source base58");
        let expected_source_base58 = json["source"]
            .as_str()
            .expect("given json .source is not a string");
        assert_eq!(&source_base58[..len], expected_source_base58.as_bytes());

        self.fee.is(&json["fee"]);
        self.counter.is(&json["counter"]);
        self.balance.is(&json["balance"]);
        self.gas_limit.is(&json["gas_limit"]);
        self.storage_limit.is(&json["storage_limit"]);

        match (
            self.delegate_base58()
                .expect("couldn't encode delegate to base58"),
            json.get("delegate")
                .map(|d| d.as_str().expect("given json .delegate is not a string")),
        ) {
            (None, Some(_)) => panic!("delegate was not parsed were it was present"),
            (Some(_), None) => panic!("delegate was parsed where it wasn't present"),
            (None, None) => {}
            (Some((len, parsed)), Some(expected)) => {
                assert_eq!(&parsed[..len], expected.as_bytes())
            }
        }

        //TODO: verify script
    }
}

#[cfg(test)]
mod tests {
    use arrayref::array_ref;

    use crate::{crypto::Curve, parser::Zarith};

    use super::{Origination, Script};

    #[test]
    fn origination() {
        const INPUT_HEX: &str = "0035e993d8c7aaa42b5e3ccd86a33390ececc73abd\
                                 904e\
                                 01\
                                 0a\
                                 0a\
                                 00\
                                 00\
                                 0000025b\
                                 020000025605000563035d050107650865046e0000000525616464720663035d00000004256b657900000005256d6772310865046e0000000525616464720663035d00000004256b657900000005256d6772320502020000020103210200000012031703160416000000082561646472204025034804420000000525402025400200000012020000000d03210316051f02000000020317020000012b03190325072c02000000810200000012020000000d03210316051f02000000020317034c02000000630321051f020000003204160000000340252502000000240321041700000004256b65790320041600000003402525044200000007254020256b6579041700000003402525034c044200000017254020254020406368616e6765645f6d6772315f6b6579020000009a032102000000060317031703160348020000008603190325072c020000006d0200000012020000000d03210316051f02000000020317034c020000004f0321051f020000003204170000000340252502000000240321041700000004256b65790320041600000003402525044200000007254020256b6579041600000003402525044200000005254020254002000000090200000004034f03270321020000000403160317051f020000000b0321020000000403170317072f0200000020072f020000000e053e035d034e053d036d034c031b02000000060320053d036d0200000049034c0200000042072f02000000060320053d036d0200000030051f02000000020321020000002203190325072c020000000c0346034e053d036d034c031b02000000060320053d036d0342\
                                 0000005c\
                                 070707070100000024747a31515a364b5937643342755a4454316431396455786f51727446504e32514a33686e030607070100000024747a31515a364b5937643342755a4454316431396455786f51727446504e32514a33686e0306";

        let input = hex::decode(INPUT_HEX).expect("invalid input hex");

        let (rem, parsed) = Origination::from_bytes(&input).expect("failed to parse endorsement");
        assert_eq!(rem.len(), 0);

        let expected = Origination {
            source: (Curve::Bip32Ed25519, array_ref!(&input, 1, 20)),
            fee: Zarith {
                bytes: &input[21..23],
                is_negative: None,
            },
            counter: Zarith {
                bytes: &input[23..24],
                is_negative: None,
            },
            gas_limit: Zarith {
                bytes: &input[24..25],
                is_negative: None,
            },
            storage_limit: Zarith {
                bytes: &input[25..26],
                is_negative: None,
            },
            balance: Zarith {
                bytes: &input[26..27],
                is_negative: None,
            },
            delegate: None,
            script: Script {
                code: &input[28 + 4..28 + 4 + 0x25b],
                storage: &input[28 + 4 + 0x25b + 4..],
            },
        };
        assert_eq!(parsed, expected);
    }
}
