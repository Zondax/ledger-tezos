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
use super::*;

use core::ptr::addr_of_mut;

#[repr(u8)] //IMPORTANT
            //see OperationType comment
enum OperationTypeKind {
    Transfer,
    Delegation,
    Endorsement,
    EndorsementWithSlot,
    Ballot,
    Reveal,
    Proposals,
    Origination,
    ActivateAccount,
    FailingNoop,
}

#[repr(C)]
struct TransferVariant<'b>(OperationTypeKind, Transfer<'b>);

#[repr(C)]
struct DelegationVariant<'b>(OperationTypeKind, Delegation<'b>);

#[repr(C)]

struct EndorsementVariant(OperationTypeKind, Endorsement);

#[repr(C)]
struct EndorsementWithSlotVariant<'b>(OperationTypeKind, EndorsementWithSlot<'b>);

#[repr(C)]
struct BallotVariant<'b>(OperationTypeKind, Ballot<'b>);

#[repr(C)]
struct RevealVariant<'b>(OperationTypeKind, Reveal<'b>);

#[repr(C)]
struct ProposalsVariant<'b>(OperationTypeKind, Proposals<'b>);

#[repr(C)]
struct OriginationVariant<'b>(OperationTypeKind, Origination<'b>);

#[repr(C)]
struct ActivateAccountVariant<'b>(OperationTypeKind, ActivateAccount<'b>);

#[repr(C)]
struct FailingNoopVariant<'b>(OperationTypeKind, FailingNoop<'b>);

#[derive(Clone, Copy)]
//ABSOLUTELY IMPORTANT, DO NOT CHANGE THIS
#[repr(u8)]
// else, run all unit tests many times + fuzzer and find an alternative way
pub enum OperationType<'b> {
    Transfer(Transfer<'b>),
    Delegation(Delegation<'b>),
    Endorsement(Endorsement),
    EndorsementWithSlot(EndorsementWithSlot<'b>),
    Ballot(Ballot<'b>),
    Reveal(Reveal<'b>),
    Proposals(Proposals<'b>),
    Origination(Origination<'b>),
    ActivateAccount(ActivateAccount<'b>),
    FailingNoop(FailingNoop<'b>),
    UnknownOp(&'b [u8]),
    #[cfg(not(test))]
    AnonymousOp(()),
    #[cfg(test)]
    AnonymousOp(AnonymousOp<'b>),
}

impl<'b> OperationType<'b> {
    pub const UNKNOWN_OP_HASH_BASE58_LEN: usize = 50;

    #[inline(never)]
    pub fn from_bytes_into(
        input: &'b [u8],
        out: &mut MaybeUninit<Self>,
    ) -> Result<&'b [u8], nom::Err<ParserError>> {
        crate::sys::zemu_log_stack("OperationType::from_bytes\x00");

        let (rem, tag) = le_u8(input)?;

        let rem = match tag {
            0x00 => {
                let out = out.as_mut_ptr() as *mut EndorsementVariant;
                //valid pointer
                let data = unsafe { &mut *addr_of_mut!((*out).1).cast() };

                let rem = Endorsement::from_bytes_into(rem, data)?;

                //pointer is valid
                unsafe {
                    addr_of_mut!((*out).0).write(OperationTypeKind::Endorsement);
                }

                rem
            }
            anon @ 0x01 | anon @ 0x02 | anon @ 0x03 => {
                let (rem, data) = AnonymousOp::from_bytes(anon, rem)?;
                *out = MaybeUninit::new(Self::AnonymousOp(data));
                rem
            }
            0x04 => {
                let out = out.as_mut_ptr() as *mut ActivateAccountVariant;
                //valid pointer
                let data = unsafe { &mut *addr_of_mut!((*out).1).cast() };

                let rem = ActivateAccount::from_bytes_into(rem, data)?;

                //pointer is valid
                unsafe {
                    addr_of_mut!((*out).0).write(OperationTypeKind::ActivateAccount);
                }
                rem
            }
            0x05 => {
                let out = out.as_mut_ptr() as *mut ProposalsVariant;
                //valid pointer
                let data = unsafe { &mut *addr_of_mut!((*out).1).cast() };

                let rem = Proposals::from_bytes_into(rem, data)?;

                //pointer is valid
                unsafe {
                    addr_of_mut!((*out).0).write(OperationTypeKind::Proposals);
                }
                rem
            }
            0x06 => {
                let out = out.as_mut_ptr() as *mut BallotVariant;
                //valid pointer
                let data = unsafe { &mut *addr_of_mut!((*out).1).cast() };

                let rem = Ballot::from_bytes_into(rem, data)?;

                //pointer is valid
                unsafe {
                    addr_of_mut!((*out).0).write(OperationTypeKind::Ballot);
                }
                rem
            }
            0x0A => {
                let out = out.as_mut_ptr() as *mut EndorsementWithSlotVariant;
                //valid pointer
                let data = unsafe { &mut *addr_of_mut!((*out).1).cast() };

                let rem = EndorsementWithSlot::from_bytes_into(rem, data)?;

                //pointer is valid
                unsafe {
                    addr_of_mut!((*out).0).write(OperationTypeKind::EndorsementWithSlot);
                }
                rem
            }
            0x11 => {
                let out = out.as_mut_ptr() as *mut FailingNoopVariant;
                //valid pointer
                let data = unsafe { &mut *addr_of_mut!((*out).1).cast() };

                let rem = FailingNoop::from_bytes_into(rem, data)?;

                //pointer is valid
                unsafe {
                    addr_of_mut!((*out).0).write(OperationTypeKind::FailingNoop);
                }
                rem
            }
            0x6B => {
                let out = out.as_mut_ptr() as *mut RevealVariant;
                //valid pointer
                let data = unsafe { &mut *addr_of_mut!((*out).1).cast() };

                let rem = Reveal::from_bytes_into(rem, data)?;

                //pointer is valid
                unsafe {
                    addr_of_mut!((*out).0).write(OperationTypeKind::Reveal);
                }
                rem
            }
            0x6C => {
                let out = out.as_mut_ptr() as *mut TransferVariant;
                //valid ptr
                let data = unsafe { &mut *addr_of_mut!((*out).1).cast() };

                let rem = Transfer::from_bytes_into(rem, data)?;

                //good ptr
                unsafe {
                    addr_of_mut!((*out).0).write(OperationTypeKind::Transfer);
                }
                rem
            }
            0x6D => {
                let out = out.as_mut_ptr() as *mut OriginationVariant;
                //valid ptr
                let data = unsafe { &mut *addr_of_mut!((*out).1).cast() };

                let rem = Origination::from_bytes_into(rem, data)?;

                //good ptr
                unsafe {
                    addr_of_mut!((*out).0).write(OperationTypeKind::Origination);
                }
                rem
            }
            0x6E => {
                let out = out.as_mut_ptr() as *mut DelegationVariant;
                //valid ptr
                let data = unsafe { &mut *addr_of_mut!((*out).1).cast() };

                let rem = Delegation::from_bytes_into(rem, data)?;

                //good ptr
                unsafe {
                    addr_of_mut!((*out).0).write(OperationTypeKind::Delegation);
                }
                rem
            }
            _ => {
                *out = MaybeUninit::new(Self::UnknownOp(rem));
                &[] as _
            }
        };

        Ok(rem)
    }

    pub fn is_transfer(&self) -> bool {
        matches!(self, OperationType::Transfer(_))
    }

    /// Returns the number of different items
    /// in a given `OperationType`
    ///
    /// Usually, the number of fields of an operaton
    pub fn ui_items(&self) -> usize {
        match self {
            Self::Transfer(tx) => tx.num_items(),
            Self::Delegation(del) => del.num_items(),
            Self::Endorsement(end) => end.num_items(),
            Self::EndorsementWithSlot(end) => end.num_items(),
            Self::Ballot(vote) => vote.num_items(),
            Self::Reveal(rev) => rev.num_items(),
            Self::Proposals(prop) => prop.num_items(),
            Self::Origination(orig) => orig.num_items(),
            Self::ActivateAccount(act) => act.num_items(),
            Self::FailingNoop(fail) => fail.num_items(),
            Self::UnknownOp(_) => 2,
            Self::AnonymousOp(_) => 0,
        }
    }

    #[inline(never)]
    fn hash_and_base58(
        input: &[u8],
    ) -> Result<(usize, [u8; OperationType::UNKNOWN_OP_HASH_BASE58_LEN]), ViewError> {
        use crate::sys::hash::{Blake2b, Hasher};

        let digest: [u8; 32] = Blake2b::digest(input).map_err(|_| ViewError::Unknown)?;

        let mut checksum = [0; 4];

        sha256x2(&[&digest[..]], &mut checksum).map_err(|_| ViewError::Unknown)?;

        let input = {
            let mut array = [0; 32 + 4];
            array[..32].copy_from_slice(&digest[..]);
            array[32..].copy_from_slice(&checksum[..]);
            array
        };

        let mut out = [0; Self::UNKNOWN_OP_HASH_BASE58_LEN];
        let len = bs58::encode(input)
            .into(&mut out[..])
            .expect("encoded in base58 is not of the right length");

        Ok((len, out))
    }

    #[inline(never)]
    pub fn render_unknown(
        input: &'b [u8],
        item_n: u8,
        title: &mut [u8],
        message: &mut [u8],
        page: u8,
    ) -> Result<u8, ViewError> {
        use crate::handlers::handle_ui_message;
        use bolos::{pic_str, PIC};

        match item_n {
            0 => {
                let title_content = pic_str!(b"Type");
                title[..title_content.len()].copy_from_slice(title_content);

                handle_ui_message(&pic_str!(b"Unknown Operation")[..], message, page)
            }
            1 => {
                let title_content = pic_str!(b"Hash");
                title[..title_content.len()].copy_from_slice(title_content);

                let (len, base58) = Self::hash_and_base58(input)?;
                handle_ui_message(&base58[..len], message, page)
            }
            _ => Err(ViewError::NoData),
        }
    }
}
