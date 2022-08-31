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
    do_parse,
    number::complete::{be_i32, be_u8},
    take, IResult,
};
use zemu_sys::ViewError;

use crate::{
    constants::tzprefix::P,
    crypto::Curve,
    handlers::{handle_ui_message, parser_common::ParserError, public_key::Addr, sha256x2},
    parser::{public_key_hash, DisplayableItem},
    utils::{bs58_encode, ApduPanic},
};

use core::{
    convert::{TryFrom, TryInto},
    mem::MaybeUninit,
    ptr::addr_of_mut,
};

const PROPOSAL_BYTES_LEN: usize = 32;

#[derive(Clone, Copy, PartialEq, Eq)]
#[cfg_attr(test, derive(Debug))]
#[repr(u8)]
pub enum Vote {
    Yay,
    Nay,
    Pass,
}

impl TryFrom<u8> for Vote {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Ok(match value {
            0 => Self::Yay,
            1 => Self::Nay,
            2 => Self::Pass,
            _ => return Err(()),
        })
    }
}

#[derive(Clone, Copy, PartialEq, Eq, property::Property)]
#[cfg_attr(test, derive(Debug))]
#[property(mut(disable), get(public), set(disable))]
pub struct Ballot<'b> {
    source: (Curve, &'b [u8; 20]),
    period: i32,
    proposal: &'b [u8; PROPOSAL_BYTES_LEN],
    vote: Vote,
}

impl<'b> Ballot<'b> {
    pub const PROPOSAL_BASE58_LEN: usize = 52;

    #[inline(never)]
    pub fn from_bytes(input: &'b [u8]) -> IResult<&[u8], Self, ParserError> {
        let (rem, (source, period, proposal, ballot)) = do_parse! {input,
            source: public_key_hash >>
            period: be_i32 >>
            proposal: take!(PROPOSAL_BYTES_LEN) >>
            vote: be_u8 >>
            (source, period, proposal, vote)
        }?;

        let proposal = arrayref::array_ref!(proposal, 0, PROPOSAL_BYTES_LEN);
        let vote = ballot
            .try_into()
            .map_err(|_| ParserError::InvalidBallotVote)?;

        Ok((
            rem,
            Self {
                source,
                period,
                proposal,
                vote,
            },
        ))
    }

    #[inline(never)]
    pub fn from_bytes_into(
        input: &'b [u8],
        out: &mut MaybeUninit<Self>,
    ) -> Result<&'b [u8], nom::Err<ParserError>> {
        let (rem, (source, period, proposal, ballot)) = do_parse! {input,
            source: public_key_hash >>
            period: be_i32 >>
            proposal: take!(PROPOSAL_BYTES_LEN) >>
            vote: be_u8 >>
            (source, period, proposal, vote)
        }?;

        let proposal = arrayref::array_ref!(proposal, 0, PROPOSAL_BYTES_LEN);
        let vote = ballot
            .try_into()
            .map_err(|_| ParserError::InvalidBallotVote)?;

        let out = out.as_mut_ptr();
        //pointer is valid and aligned
        // we are only writing to uninit memory, not ready
        unsafe {
            addr_of_mut!((*out).source).write(source);
            addr_of_mut!((*out).period).write(period);
            addr_of_mut!((*out).proposal).write(proposal);
            addr_of_mut!((*out).vote).write(vote);
        }

        Ok(rem)
    }

    #[inline(never)]
    pub fn proposal_base58(
        &self,
    ) -> Result<(usize, [u8; Ballot::PROPOSAL_BASE58_LEN]), bolos::Error> {
        let mut checksum = [0; 4];

        sha256x2(&[P, &self.proposal[..]], &mut checksum)?;

        let input = {
            let mut array = [0; 2 + PROPOSAL_BYTES_LEN + 4];
            array[..2].copy_from_slice(P);
            array[2..2 + PROPOSAL_BYTES_LEN].copy_from_slice(&self.proposal[..]);
            array[2 + PROPOSAL_BYTES_LEN..].copy_from_slice(&checksum[..]);
            array
        };

        let mut out = [0; Self::PROPOSAL_BASE58_LEN];
        let len = bs58_encode(input, &mut out[..])
            .apdu_expect("encoded in base58 is not of the right length");

        Ok((len, out))
    }
}

impl<'b> DisplayableItem for Ballot<'b> {
    fn num_items(&self) -> usize {
        1 + 4
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

                handle_ui_message(&pic_str!(b"Ballot")[..], message, page)
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
            //Period
            2 => {
                let title_content = pic_str!(b"Period");
                title[..title_content.len()].copy_from_slice(title_content);

                let mut zarith_buf = [0; i32::FORMATTED_SIZE_DECIMAL];

                handle_ui_message(itoa(self.period, &mut zarith_buf), message, page)
            }
            //Proposal
            3 => {
                let title_content = pic_str!(b"Proposal");
                title[..title_content.len()].copy_from_slice(title_content);

                let (len, mex) = self.proposal_base58().map_err(|_| ViewError::Unknown)?;

                handle_ui_message(&mex[..len], message, page)
            }
            //Vote
            4 => {
                let title_content = pic_str!(b"Vote");
                title[..title_content.len()].copy_from_slice(title_content);

                let mex = match self.vote {
                    Vote::Yay => pic_str!("yay"),
                    Vote::Nay => pic_str!("nay"),
                    Vote::Pass => pic_str!("pass"),
                };

                handle_ui_message(mex.as_bytes(), message, page)
            }
            _ => Err(ViewError::NoData),
        }
    }
}

#[cfg(test)]
impl<'b> Ballot<'b> {
    fn source_base58(&self) -> Result<(usize, [u8; Addr::BASE58_LEN]), bolos::Error> {
        let source = self.source;
        let addr = Addr::from_hash(source.1, source.0)?;

        Ok(addr.base58())
    }

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

        let vote = json["ballot"]
            .as_str()
            .expect("given json .ballot is not a string");

        match (self.vote, vote) {
            (Vote::Yay, "yay") | (Vote::Nay, "nay") | (Vote::Pass, "pass") => {}
            (parsed, got) => panic!("parsed ballot was {:?}; expected {}", parsed, got),
        }

        let (len, proposal_base58) = self
            .proposal_base58()
            .expect("couldn't compute proposal base58");

        let expected_proposal_base58 = json["proposal"]
            .as_str()
            .expect("given json .proposal is not a string");
        assert_eq!(&proposal_base58[..len], expected_proposal_base58.as_bytes());
    }
}

#[cfg(test)]
mod tests {
    use crate::crypto::Curve;

    use super::{Ballot, Vote};

    #[test]
    fn ballot() {
        const INPUT_HEX: &str = "0035e993d8c7aaa42b5e3ccd86a33390ececc73abd\
                                 fffffed4\
                                 3e5e3a606afab74a59ca09e333633e2770b6492c5e594455b71e9a2f0ea92afb\
                                 00";

        let input = hex::decode(INPUT_HEX).expect("invalid input hex");

        let (rem, parsed) = Ballot::from_bytes(&input).expect("failed to parse endorsement");
        assert_eq!(rem.len(), 0);

        let expected = Ballot {
            source: (Curve::Bip32Ed25519, arrayref::array_ref!(input, 1, 20)),
            period: -300,
            proposal: arrayref::array_ref!(input, 25, 32),
            vote: Vote::Yay,
        };
        assert_eq!(parsed, expected);
    }
}
