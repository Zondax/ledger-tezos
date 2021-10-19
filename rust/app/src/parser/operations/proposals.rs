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
use nom::{
    do_parse,
    number::complete::{be_i32, be_u32},
    take, IResult,
};
use zemu_sys::ViewError;

use crate::{
    constants::tzprefix::P,
    crypto::Curve,
    handlers::{handle_ui_message, parser_common::ParserError, public_key::Addr, sha256x2},
    parser::{public_key_hash, DisplayableItem},
};

const PROPOSAL_BYTES_LEN: usize = 32;

#[derive(Clone, Copy, PartialEq, Eq, property::Property)]
#[property(mut(disable), get(public), set(disable))]
pub struct Proposals<'b> {
    source: (Curve, &'b [u8; 20]),
    period: i32,
    proposals: &'b [[u8; PROPOSAL_BYTES_LEN]],
}

impl<'b> Proposals<'b> {
    pub const PROPOSAL_BASE58_LEN: usize = 52;

    #[inline(never)]
    pub fn from_bytes(input: &'b [u8]) -> IResult<&[u8], Self, ParserError> {
        let (rem, (source, period, proposals)) = do_parse! {input,
            source: public_key_hash >>
            period: be_i32 >>
            proposals_len: be_u32 >>
            proposals: take!(proposals_len) >>
            (source, period, proposals)
        }?;

        let proposals =
            bytemuck::try_cast_slice(proposals).map_err(|_| ParserError::ProposalsLengthInvalid)?;

        Ok((
            rem,
            Self {
                source,
                period,
                proposals,
            },
        ))
    }

    #[inline(never)]
    pub fn from_bytes_into(
        input: &'b [u8],
        out: &mut MaybeUninit<Self>,
    ) -> Result<&'b [u8], nom::Err<ParserError>> {
        let (rem, (source, period, proposals)) = do_parse! {input,
            source: public_key_hash >>
            period: be_i32 >>
            proposals_len: be_u32 >>
            proposals: take!(proposals_len) >>
            (source, period, proposals)
        }?;

        let proposals =
            bytemuck::try_cast_slice(proposals).map_err(|_| ParserError::ProposalsLengthInvalid)?;

        let out = out.as_mut_ptr();
        //dereferencing pointer from references is okay
        // also we are never reading this "uninitialized" memory
        unsafe {
            addr_of_mut!((*out).source).write(source);
            addr_of_mut!((*out).period).write(period);
            addr_of_mut!((*out).proposals).write(proposals);
        }

        Ok(rem)
    }

    #[inline(never)]
    pub fn proposal_base58(
        proposal: &[u8; PROPOSAL_BYTES_LEN],
    ) -> Result<(usize, [u8; Proposals::PROPOSAL_BASE58_LEN]), bolos::Error> {
        let mut checksum = [0; 4];

        sha256x2(&[P, &proposal[..]], &mut checksum)?;

        let input = {
            let mut array = [0; 2 + PROPOSAL_BYTES_LEN + 4];
            array[..2].copy_from_slice(P);
            array[2..2 + PROPOSAL_BYTES_LEN].copy_from_slice(&proposal[..]);
            array[2 + PROPOSAL_BYTES_LEN..].copy_from_slice(&checksum[..]);
            array
        };

        let mut out = [0; Self::PROPOSAL_BASE58_LEN];
        let len = bs58::encode(input)
            .into(&mut out[..])
            .expect("encoded in base58 is not of the right length");

        Ok((len, out))
    }

    fn source_base58(&self) -> Result<(usize, [u8; Addr::BASE58_LEN]), bolos::Error> {
        let source = self.source;
        let addr = Addr::from_hash(source.1, source.0)?;

        Ok(addr.base58())
    }
}

impl<'b> DisplayableItem for Proposals<'b> {
    fn num_items(&self) -> usize {
        1 + 2 + self.proposals.len()
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

                handle_ui_message(&pic_str!(b"Proposals")[..], message, page)
            }
            //source
            1 => {
                let title_content = pic_str!(b"Source");
                title[..title_content.len()].copy_from_slice(title_content);

                let (len, mex) = self.source_base58().map_err(|_| ViewError::Unknown)?;
                handle_ui_message(&mex[..len], message, page)
            }
            //Period
            2 => {
                let title_content = pic_str!(b"Period");
                title[..title_content.len()].copy_from_slice(title_content);

                let mut zarith_buf = [0; i32::FORMATTED_SIZE_DECIMAL];

                handle_ui_message(itoa(self.period, &mut zarith_buf), message, page)
            }
            //Proposal
            n if n - 3 < self.proposals.len() as u8 => {
                //-3 because we need to account for the previous pages
                //and we'll also use this n to select the proposal
                let n = n - 3;

                //convert n to string
                let (proposal_n, buf_len) = {
                    let mut buf = [0; u8::FORMATTED_SIZE_DECIMAL];
                    //n + 1 since we want tho show starting from 1 and not 0
                    let len = itoa(n + 1, &mut buf).len();
                    (buf, len)
                };

                //prepare page title
                let title_content = pic_str!(b"Proposal #"!);
                let title_len = title_content.len();
                title[..title_len].copy_from_slice(&title_content[..]);
                if title_len + buf_len + 1 < title.len() {
                    //copy the index of the proposal
                    title[title_len..title_len + buf_len].copy_from_slice(&proposal_n[..buf_len]);
                    title[title_len + buf_len] = 0; //null terminate
                } else {
                    //if it won't fit anymore then we put a ".." to show something
                    // we know this fits
                    title[title_len..title_len + 3].copy_from_slice(&pic_str!(b"..")[..]);
                }

                //get base58 of the proposal data
                let (len, mex) = Self::proposal_base58(&self.proposals[n as usize])
                    .map_err(|_| ViewError::Unknown)?;

                handle_ui_message(&mex[..len], message, page)
            }
            _ => Err(ViewError::NoData),
        }
    }
}

#[cfg(test)]
impl<'b> Proposals<'b> {
    pub fn is(&self, json: &serde_json::Map<std::string::String, serde_json::Value>) {
        let (len, source_base58) = self
            .source_base58()
            .expect("couldn't compute source base58");
        let expected_source_base58 = json["source"]
            .as_str()
            .expect("given json .source is not a string");

        assert_eq!(&source_base58[..len], expected_source_base58.as_bytes());

        let period = json["period"]
            .as_i64()
            .expect("given json .level is not a signed integer");

        assert_eq!(self.period, period as i32);

        let expected_props_base58 = json["proposals"]
            .as_array()
            .expect("given json .proposals is not an array");
        for (i, prop) in self.proposals.iter().enumerate() {
            let (len, prop_base58) = Self::proposal_base58(prop)
                .unwrap_or_else(|_| panic!("couldn't encode proposal #{} as base58", i));

            let expected_prop_base58 = expected_props_base58[i]
                .as_str()
                .unwrap_or_else(|| panic!("given json .proposals[{}] was not a string", i));

            assert_eq!(&prop_base58[..len], expected_prop_base58.as_bytes());
        }
    }
}

#[cfg(test)]
mod tests {
    use arrayref::array_ref;

    use crate::{crypto::Curve, parser::operations::Proposals};

    #[test]
    fn proposals() {
        const INPUT_HEX: &str = "0035e993d8c7aaa42b5e3ccd86a33390ececc73abd\
                                 000063ce\
                                 00000040\
                                 3e5e3a606afab74a59ca09e333633e2770b6492c5e594455b71e9a2f0ea92afb\
                                 3e5e3a606afab74a59ca09e333633e2770b6492c5e594455b71e9a2f0ea92afb";

        let input = hex::decode(INPUT_HEX).expect("invalid input hex");

        let (rem, parsed) = Proposals::from_bytes(&input).expect("failed to parse endorsement");
        assert_eq!(rem.len(), 0);

        let proposals_bytes_len = {
            let len = array_ref!(&input, 25, 4);
            u32::from_be_bytes(*len) as usize
        };
        let proposals = bytemuck::cast_slice(&input[29..29 + proposals_bytes_len]);

        let expected = Proposals {
            source: (Curve::Bip32Ed25519, arrayref::array_ref!(input, 1, 20)),
            period: 25550,
            proposals,
        };

        assert_eq!(parsed, expected);
    }
}
