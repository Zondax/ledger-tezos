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
use nom::{bytes::complete::take, number::complete::le_u8, Finish, IResult};
use zemu_sys::ViewError;

use crate::{
    constants::tzprefix::{B, KT1, TZ1, TZ2, TZ3},
    crypto::Curve,
    handlers::{parser_common::ParserError, sha256x2},
};

use core::mem::MaybeUninit;

use super::{public_key_hash, DisplayableItem};

#[derive(Clone, Copy, property::Property)]
#[property(get(public), mut(public), set(disable))]
pub struct Operation<'b> {
    #[property(get(disable), mut(disable))]
    branch: &'b [u8; 32],

    ops: EncodedOperations<'b>,
}

impl<'b> Operation<'b> {
    pub const BASE58_BRANCH_LEN: usize = 52;

    #[inline(never)]
    pub fn new(input: &'b [u8]) -> Result<Self, ParserError> {
        crate::sys::zemu_log_stack("Operation::new\x00");
        let (rem, branch) = take::<_, _, ParserError>(32usize)(input).finish()?;
        let branch = arrayref::array_ref!(branch, 0, 32);

        Ok(Self {
            branch,
            ops: EncodedOperations::new(rem),
        })
    }

    pub fn branch(&self) -> &'b [u8; 32] {
        self.branch
    }

    #[inline(never)]
    pub fn get_base58_branch(
        &self,
    ) -> Result<(usize, [u8; Operation::BASE58_BRANCH_LEN]), bolos::Error> {
        Self::base58_branch(self.branch)
    }

    pub fn base58_branch(
        branch: &[u8; 32],
    ) -> Result<(usize, [u8; Operation::BASE58_BRANCH_LEN]), bolos::Error> {
        let mut out = [0; Self::BASE58_BRANCH_LEN];

        Self::base58_branch_into(branch, &mut out).map(|n| (n, out))
    }

    #[inline(never)]
    pub fn base58_branch_into(
        branch: &[u8; 32],
        out: &mut [u8; Operation::BASE58_BRANCH_LEN],
    ) -> Result<usize, bolos::Error> {
        let mut checksum = [0; 4];

        sha256x2(&[B, &branch[..]], &mut checksum)?;

        let mut array = [0; 2 + 32 + 4];
        array[..2].copy_from_slice(B);
        array[2..2 + 32].copy_from_slice(&branch[..]);
        array[2 + 32..].copy_from_slice(&checksum[..]);

        let len = bs58::encode(array)
            .into(&mut out[..])
            .expect("encoded in base58 is not of the right length");

        Ok(len)
    }
}

#[derive(Clone, Copy)]
pub struct EncodedOperations<'b> {
    source: &'b [u8],
    //number of bytes read
    read: usize,
}

impl<'b> EncodedOperations<'b> {
    pub fn new(source: &'b [u8]) -> Self {
        Self { source, read: 0 }
    }

    #[inline(never)]
    fn parse_into(
        &self,
        loc: &mut MaybeUninit<OperationType<'b>>,
    ) -> Result<Option<usize>, nom::Err<ParserError>> {
        crate::sys::zemu_log_stack("EncodedOperation::parse\x00");
        let input = &self.source[self.read..];
        let input_len = input.len();

        if input_len == 0 {
            return Ok(None);
        }

        let rem = match OperationType::from_bytes_into(input, loc) {
            Ok(ok) => ok,
            //there was some remaing data but it's probably the signature
            // since we don't recognize the operation tag
            Err(nom::Err::Error(ParserError::UnknownOperation)) if input_len == 64 => {
                return Ok(None)
            }
            Err(err) => return Err(err),
        };

        //calculate the number of bytes read based
        // on the number of bytes left in the remaning section
        //this will also take into account the bytes removed earlier
        // to skip already read bytes
        let read = self.source.len() - rem.len();

        Ok(Some(read))
    }

    pub fn peek_next(&self) -> Result<Option<OperationType<'b>>, nom::Err<ParserError>> {
        let mut out = MaybeUninit::uninit();

        match self.parse_into(&mut out) {
            Ok(Some(_)) => Ok(Some(unsafe { out.assume_init() })),
            Ok(None) => Ok(None),
            Err(err) => Err(err),
        }
    }

    pub fn parse_next(&mut self) -> Result<Option<OperationType<'b>>, nom::Err<ParserError>> {
        let mut out = MaybeUninit::uninit();

        match self.parse_into(&mut out) {
            Ok(None) => Ok(None),
            Err(err) => Err(err),
            Ok(Some(read)) => {
                self.read = read;
                Ok(Some(unsafe { out.assume_init() }))
            }
        }
    }

    pub fn parse_next_into(
        &mut self,
        out: &mut MaybeUninit<OperationType<'b>>,
    ) -> Result<Option<()>, nom::Err<ParserError>> {
        match self.parse_into(out) {
            Ok(None) => Ok(None),
            Err(err) => Err(err),
            Ok(Some(read)) => {
                self.read = read;
                Ok(Some(()))
            }
        }
    }

    pub fn source_index(&self) -> usize {
        self.read
    }

    /// Sets the inner index to the specified one.
    ///
    /// # Safety
    /// If the specified "read" argument is a byte in the middle of an operation
    /// it will make further reading impossible
    pub unsafe fn set_source_index(&mut self, read: usize) {
        self.read = read;
    }
}

mod operation_type;
pub use operation_type::OperationType;

mod activate_account;
mod ballot;
mod delegation;
mod double_baking_evidence;
mod endorsement;
mod failing_noop;
mod origination;
mod proposals;
mod reveal;
mod seed_nonce_revelation;
mod transfer;

pub use activate_account::ActivateAccount;
pub use ballot::Ballot;
pub use delegation::Delegation;
pub use double_baking_evidence::DoubleBakingEvidence;
pub use endorsement::{DoubleEndorsementEvidence, Endorsement, EndorsementWithSlot};
pub use failing_noop::FailingNoop;
pub use origination::Origination;
pub use proposals::Proposals;
pub use reveal::Reveal;
pub use seed_nonce_revelation::SeedNonceRevelation;
pub use transfer::Transfer;

#[derive(Clone, Copy)]
pub enum AnonymousOp<'b> {
    DoubleEndorsementEvidence(DoubleEndorsementEvidence<'b>),
    SeedNonceRevelation(SeedNonceRevelation<'b>),
    DoubleBakingEvidence(DoubleBakingEvidence<'b>),
}

impl<'b> AnonymousOp<'b> {
    #[inline(never)]
    #[cfg(not(test))]
    pub fn from_bytes(tag: u8, rem: &'b [u8]) -> Result<(&'b [u8], ()), nom::Err<ParserError>> {
        crate::sys::zemu_log_stack("AnonymousOp::from_bytes\x00");
        let rem = match tag {
            0x01 => {
                let (rem, _) = SeedNonceRevelation::from_bytes(rem)?;
                rem
            }
            0x02 => {
                let (rem, _) = DoubleEndorsementEvidence::from_bytes(rem)?;
                rem
            }
            0x03 => {
                let (rem, _) = DoubleBakingEvidence::from_bytes(rem)?;
                rem
            }
            _ => return Err(ParserError::UnknownOperation.into()),
        };

        Ok((rem, ()))
    }

    #[inline(never)]
    #[cfg(test)]
    pub fn from_bytes(tag: u8, rem: &'b [u8]) -> Result<(&'b [u8], Self), nom::Err<ParserError>> {
        crate::sys::zemu_log_stack("AnonymousOp::from_bytes\x00");
        let (rem, data) = match tag {
            0x01 => {
                let (rem, data) = SeedNonceRevelation::from_bytes(rem)?;
                (rem, Self::SeedNonceRevelation(data))
            }
            0x02 => {
                let (rem, data) = DoubleEndorsementEvidence::from_bytes(rem)?;
                (rem, Self::DoubleEndorsementEvidence(data))
            }
            0x03 => {
                let (rem, data) = DoubleBakingEvidence::from_bytes(rem)?;
                (rem, Self::DoubleBakingEvidence(data))
            }
            _ => return Err(ParserError::UnknownOperation.into()),
        };

        Ok((rem, data))
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ContractID<'b> {
    Implicit(Curve, &'b [u8; 20]),
    Originated(&'b [u8; 20]),
}

impl<'b> ContractID<'b> {
    pub const BASE58_LEN: usize = 37;

    #[cfg(test)]
    fn from_bytes(input: &'b [u8]) -> IResult<&[u8], Self, ParserError> {
        use nom::{dbg_basic, take, tuple as tuplem};
        use std::{eprintln, println};

        let (rem, tag) = dbg_basic!(input, le_u8)?;
        match tag {
            0x00 => {
                let (rem, (crv, hash)) = public_key_hash(rem)?;
                Ok((rem, Self::Implicit(crv, hash)))
            }
            0x01 => {
                //discard last byte (padding)
                let (rem, (hash, _)) = dbg_basic!(rem, tuplem!(take!(20usize), le_u8))?;
                let hash = arrayref::array_ref!(hash, 0, 20);
                Ok((rem, Self::Originated(hash)))
            }
            err => {
                eprintln!(
                    "found {:x} at {}; {:x?}",
                    err,
                    input.len() - rem.len(),
                    input
                );
                Err(ParserError::parser_invalid_address.into())
            }
        }
    }

    #[cfg(not(test))]
    fn from_bytes(input: &'b [u8]) -> IResult<&[u8], Self, ParserError> {
        let (rem, tag) = le_u8(input)?;
        match tag {
            0x00 => {
                let (rem, (crv, hash)) = public_key_hash(rem)?;
                Ok((rem, Self::Implicit(crv, hash)))
            }
            0x01 => {
                //discard last byte (padding)
                let (rem, (hash, _)) = nom::sequence::tuple((take(20usize), le_u8))(rem)?;
                let hash = arrayref::array_ref!(hash, 0, 20);
                Ok((rem, Self::Originated(hash)))
            }
            _ => Err(ParserError::parser_invalid_address.into()),
        }
    }

    pub fn hash(&self) -> &[u8; 20] {
        match self {
            ContractID::Implicit(_, h) | ContractID::Originated(h) => h,
        }
    }

    #[inline(never)]
    pub fn base58(&self) -> Result<(usize, [u8; ContractID::BASE58_LEN]), bolos::Error> {
        let (prefix, hash) = match *self {
            Self::Originated(h) => (KT1, h),
            Self::Implicit(Curve::Bip32Ed25519 | Curve::Ed25519, h) => (TZ1, h),
            Self::Implicit(Curve::Secp256K1, h) => (TZ2, h),
            Self::Implicit(Curve::Secp256R1, h) => (TZ3, h),
        };

        let mut checksum = [0; 4];
        sha256x2(&[prefix, &hash[..]], &mut checksum)?;

        let input = {
            let mut array = [0; 3 + 20 + 4];
            array[..3].copy_from_slice(prefix);
            array[3..3 + 20].copy_from_slice(&hash[..]);
            array[3 + 20..].copy_from_slice(&checksum[..]);
            array
        };

        let mut out = [0; Self::BASE58_LEN];
        let len = bs58::encode(input)
            .into(&mut out[..])
            .expect("encoded in base58 is not the right length");

        Ok((len, out))
    }

    pub fn is_implicit(&self) -> bool {
        matches! {self, Self::Implicit(_, _)}
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        crypto::Curve,
        handlers::public_key::Addr,
        parser::operations::{ContractID, Operation, OperationType},
    };

    #[test]
    fn contract_id_pkh() {
        const INPUT_HEX: &str = "000035e993d8c7aaa42b5e3ccd86a33390ececc73abd";
        const PKH_BASE58: &str = "tz1QZ6KY7d3BuZDT1d19dUxoQrtFPN2QJ3hn";

        let input = hex::decode(INPUT_HEX).expect("invalid input hex");

        let (rem, parsed) =
            ContractID::from_bytes(&input).expect("failed to parse contract id input");

        assert_eq!(rem.len(), 0);
        assert_eq!(
            parsed,
            ContractID::Implicit(Curve::Bip32Ed25519, arrayref::array_ref!(input, 2, 20))
        );

        let addr = Addr::from_hash(parsed.hash(), Curve::Bip32Ed25519).unwrap();
        let (len, addr) = addr.base58();
        assert_eq!(&addr[..len], PKH_BASE58.as_bytes());
    }

    #[test]
    fn contract_id_contract() {
        const INPUT_HEX: &str = "016a7d4a43f51be0934a441fba4f13f9beaa47575100";
        const CONTRACT_BASE58: &str = "KT1JHqHQdHSgWBKo6H4UfG8dw3JnZSyjGkHA";

        let input = hex::decode(INPUT_HEX).expect("invalid input hex");

        let (rem, parsed) =
            ContractID::from_bytes(&input).expect("failed to parse contract id input");

        assert_eq!(rem.len(), 0);
        assert_eq!(
            parsed,
            ContractID::Originated(arrayref::array_ref!(input, 1, 20))
        );

        let (len, cid) = parsed
            .base58()
            .expect("couldn't encode contract id to base 58");
        assert_eq!(&cid[..len], CONTRACT_BASE58.as_bytes());
    }

    #[test]
    fn operation() {
        const INPUT_HEX: &str = "a99b946c97ada0f42c1bdeae0383db7893351232a832d00d0cd716eb6f66e5616c0035e993d8c7aaa42b5e3ccd86a33390ececc73abd904e010a0ae807000035e993d8c7aaa42b5e3ccd86a33390ececc73abdff02000000070a000000020202";
        const BRANCH_BASE58: &str = "BLzyjjHKEKMULtvkpSHxuZxx6ei6fpntH2BTkYZiLgs8zLVstvX";

        let input = hex::decode(INPUT_HEX).expect("invalid input hex");
        let mut parsed = Operation::new(&input).expect("couldn't parse branch");

        let (len, branch) = parsed
            .get_base58_branch()
            .expect("couldn't encode branch to base58");
        assert_eq!(&branch[..len], BRANCH_BASE58.as_bytes());

        let ops = parsed.mut_ops();
        let op = ops
            .parse_next()
            .expect("failed to parse operation")
            .expect("no next operation found");

        match op {
            OperationType::Transfer(_) => {
                //we don't check transfer here to avoid redundancy
            }
            #[allow(unreachable_patterns)]
            opt => panic!("not the expected operation type, found: {:x?}", opt),
        }

        match ops.parse_next().expect("failed to parse operation") {
            None => {}
            Some(s) => panic!("expected no operations, found {:x?}", s),
        }
    }

    #[test]
    fn operations() {
        const INPUT_HEX: &str = "a99b946c97ada0f42c1bdeae0383db7893351232a832d00d0cd716eb6f66e561\
                                 6c0035e993d8c7aaa42b5e3ccd86a33390ececc73abd904e010a0ae807000035e993d8c7aaa42b5e3ccd86a33390ececc73abdff02000000070a000000020202\
                                 6c0035e993d8c7aaa42b5e3ccd86a33390ececc73abd904e010a0ae807016a7d4a43f51be0934a441fba4f13f9beaa4757510000\
                                 6c0035e993d8c7aaa42b5e3ccd86a33390ececc73abd904e010a0ae807016a7d4a43f51be0934a441fba4f13f9beaa47575100ff03000000290100000024747a31515a364b5937643342755a4454316431396455786f51727446504e32514a33686e";
        const BRANCH_BASE58: &str = "BLzyjjHKEKMULtvkpSHxuZxx6ei6fpntH2BTkYZiLgs8zLVstvX";

        let input = hex::decode(INPUT_HEX).expect("invalid input hex");
        let mut parsed = Operation::new(&input).expect("couldn't parse branch");

        let (len, branch) = parsed
            .get_base58_branch()
            .expect("couldn't encode branch to base58");
        assert_eq!(&branch[..len], BRANCH_BASE58.as_bytes());

        let ops = parsed.mut_ops();
        let op1 = ops
            .parse_next()
            .expect("failed to parse operation")
            .expect("no next operation found");
        let op2 = ops
            .parse_next()
            .expect("failed to parse operation")
            .expect("no next operation found");
        let op3 = ops
            .parse_next()
            .expect("failed to parse operation")
            .expect("no next operation found");

        match (op1, op2, op3) {
            (
                OperationType::Transfer(_),
                OperationType::Transfer(_),
                OperationType::Transfer(_),
            ) => {
                //we don't check transfer here to avoid redundancy
            }
            #[allow(unreachable_patterns)]
            opt => panic!("not the expected operation type, found: {:x?}", opt),
        }
    }
}
