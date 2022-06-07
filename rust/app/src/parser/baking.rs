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
mod tenderbake;
pub use tenderbake::{EndorsementType, TenderbakeEndorsement};

mod emmy;
pub use emmy::EmmyEndorsement;

use crate::{
    handlers::{handle_ui_message, hwm::WaterMark, parser_common::ParserError},
    utils::ApduPanic,
};
use bolos::{pic_str, PIC};
use zemu_sys::ViewError;

use nom::{
    bytes::complete::take,
    number::complete::{be_u32, be_u64, le_u8},
    IResult,
};

use self::{emmy::EmmyFitness, tenderbake::TenderbakeFitness};

use super::DisplayableItem;

#[derive(Clone)]
#[cfg_attr(test, derive(Debug))]
pub struct BlockData<'b> {
    pub chain_id: u32,
    pub level: u32,
    pub proto: u8,
    pub predecessor: &'b [u8; 32],
    pub timestamp: u64,
    pub validation_pass: u8,
    pub operation_hash: &'b [u8; 32],
    pub fitness: Fitness<'b>,
}

impl<'b> BlockData<'b> {
    #[inline(never)]
    pub fn from_bytes(bytes: &'b [u8]) -> IResult<&[u8], Self, ParserError> {
        let (rem, chain_id) = be_u32(bytes)?;
        let (rem, level) = be_u32(rem)?;
        let (rem, proto) = le_u8(rem)?;
        let (rem, predecessor) = take(32usize)(rem)?;
        let predecessor = arrayref::array_ref!(predecessor, 0, 32);

        let (rem, timestamp) = be_u64(rem)?;
        let (rem, validation_pass) = le_u8(rem)?;
        let (rem, operation_hash) = take(32usize)(rem)?;
        let operation_hash = arrayref::array_ref!(operation_hash, 0, 32);

        let (rem, fitness_size) = be_u32(rem)?;
        //TODO: figure out what this field is
        let (rem, _pad) = take(4usize)(rem)?;
        let (_, fitness) = take(fitness_size)(rem)?;

        let (rem, fitness) = Fitness::from_bytes(fitness)?;

        Ok((
            rem,
            Self {
                chain_id,
                level,
                proto,
                predecessor,
                timestamp,
                validation_pass,
                operation_hash,
                fitness,
            },
        ))
    }

    #[inline(never)]
    pub fn validate_with_watermark(&self, hw: &WaterMark) -> bool {
        WaterMark::is_valid_blocklevel(self.level)
            && match (hw, &self.fitness) {
                (WaterMark::Emmy { level, .. }, Fitness::Emmy(_)) => self.level > *level,
                //the block is invalid if the stored watermark is tenderbake already
                (WaterMark::Tenderbake { .. }, Fitness::Emmy(_)) => false,
                //stored watermark is Emmy, and this is tenderbake
                // so we know this is always higher
                (WaterMark::Emmy { .. }, Fitness::Tenderbake(_)) => true,
                //higher level OR same level with higher round
                (
                    WaterMark::Tenderbake { level, round, .. },
                    Fitness::Tenderbake(TenderbakeFitness {
                        round: self_round, ..
                    }),
                ) => self.level > *level || (self.level == *level && self_round > round),
            }
    }

    pub fn derive_watermark(&self) -> WaterMark {
        match self.fitness {
            Fitness::Emmy(_) => WaterMark::Emmy {
                level: self.level,
                had_endorsement: false,
            },
            Fitness::Tenderbake(TenderbakeFitness { round, .. }) => WaterMark::Tenderbake {
                level: self.level,
                had_endorsement: false,
                round,
                had_preendorsement: false,
            },
        }
    }
}

impl<'b> DisplayableItem for BlockData<'b> {
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
        use lexical_core::{write as itoa, Number};

        match item_n {
            0 => {
                let title_content = pic_str!(b"Type");
                title[..title_content.len()].copy_from_slice(title_content);

                handle_ui_message(&pic_str!(b"Blocklevel")[..], message, page)
            }
            1 => {
                let title_content = pic_str!(b"ChainID");
                title[..title_content.len()].copy_from_slice(title_content);

                let mut itoa_buf = [0u8; u32::FORMATTED_SIZE_DECIMAL];

                handle_ui_message(itoa(self.chain_id, &mut itoa_buf), message, page)
            }
            2 => {
                let title_content = pic_str!(b"Blocklevel");
                title[..title_content.len()].copy_from_slice(title_content);

                let mut itoa_buf = [0u8; u32::FORMATTED_SIZE_DECIMAL];

                handle_ui_message(itoa(self.level, &mut itoa_buf), message, page)
            }
            _ => Err(ViewError::NoData),
        }
    }
}

#[derive(Clone)]
#[cfg_attr(test, derive(Debug))]
pub enum Fitness<'b> {
    Emmy(EmmyFitness<'b>),
    Tenderbake(TenderbakeFitness<'b>),
}

impl<'b> Fitness<'b> {
    pub fn round(&self) -> u32 {
        match self {
            Fitness::Emmy(_) => 0,
            Fitness::Tenderbake(tb) => tb.round,
        }
    }

    #[inline(never)]
    pub fn from_bytes(bytes: &'b [u8]) -> IResult<&[u8], Self, ParserError> {
        match (
            EmmyFitness::from_bytes(bytes),
            TenderbakeFitness::from_bytes(bytes),
        ) {
            (Ok((rem, fitness)), Err(_)) => Ok((rem, Self::Emmy(fitness))),
            (Err(_), Ok((rem, fitness))) => Ok((rem, Self::Tenderbake(fitness))),
            (Err(_), Err(_)) => Err(ParserError::InvalidProtocolVersion.into()),
            (Ok(_), Ok(_)) => unreachable!(),
        }
    }
}

pub enum EndorsementData<'b> {
    Emmy(EmmyEndorsement<'b>),
    Tenderbake(TenderbakeEndorsement<'b>),
}

impl<'b> EndorsementData<'b> {
    pub fn is_tenderbake(&self) -> bool {
        matches!(self, Self::Tenderbake(_))
    }

    pub fn endorsement_type(&self) -> &'static [u8] {
        match self {
            EndorsementData::Tenderbake(TenderbakeEndorsement {
                ty: EndorsementType::PreEndorsement,
                ..
            }) => pic_str!(b"Preendorsement"),
            _ => pic_str!(b"Endorsement"),
        }
    }

    pub fn branch(&self) -> &[u8; 32] {
        match self {
            EndorsementData::Emmy(EmmyEndorsement { branch, .. })
            | EndorsementData::Tenderbake(TenderbakeEndorsement { branch, .. }) => branch,
        }
    }

    pub fn chain_id(&self) -> u32 {
        match self {
            EndorsementData::Emmy(EmmyEndorsement { chain_id, .. })
            | EndorsementData::Tenderbake(TenderbakeEndorsement { chain_id, .. }) => *chain_id,
        }
    }

    pub fn level(&self) -> u32 {
        match self {
            EndorsementData::Emmy(EmmyEndorsement { level, .. })
            | EndorsementData::Tenderbake(TenderbakeEndorsement { level, .. }) => *level,
        }
    }

    pub fn round(&self) -> Option<u32> {
        match self {
            EndorsementData::Emmy(_) => None,
            EndorsementData::Tenderbake(TenderbakeEndorsement { round, .. }) => Some(*round),
        }
    }

    #[inline(never)]
    pub fn from_bytes(bytes: &'b [u8]) -> IResult<&[u8], Self, ParserError> {
        match EmmyEndorsement::from_bytes(bytes) {
            Ok((rem, emmy)) => Ok((rem, Self::Emmy(emmy))),
            Err(_) => match TenderbakeEndorsement::from_bytes(bytes) {
                Ok((rem, tenderbake)) => Ok((rem, Self::Tenderbake(tenderbake))),
                Err(err) => Err(err),
            },
        }
    }

    pub fn validate_with_watermark(&self, hw: &WaterMark) -> bool {
        match self {
            EndorsementData::Emmy(emmy) => emmy.validate_with_watermark(hw),
            EndorsementData::Tenderbake(tb) => tb.validate_with_watermark(hw),
        }
    }

    pub fn derive_watermark(&self) -> WaterMark {
        match self {
            EndorsementData::Emmy(EmmyEndorsement { level, .. }) => WaterMark::Emmy {
                level: *level,
                had_endorsement: true,
            },
            EndorsementData::Tenderbake(TenderbakeEndorsement {
                level,
                round,
                ty: EndorsementType::Endorsement,
                ..
            }) => WaterMark::Tenderbake {
                level: *level,
                had_endorsement: true,
                round: *round,
                had_preendorsement: true,
            },
            EndorsementData::Tenderbake(TenderbakeEndorsement {
                level,
                round,
                ty: EndorsementType::PreEndorsement,
                ..
            }) => WaterMark::Tenderbake {
                level: *level,
                had_endorsement: false,
                round: *round,
                had_preendorsement: true,
            },
        }
    }
}

impl<'b> DisplayableItem for EndorsementData<'b> {
    fn num_items(&self) -> usize {
        1 + 3 + self.is_tenderbake() as usize
    }

    #[inline(never)]
    fn render_item(
        &self,
        item_n: u8,
        title: &mut [u8],
        message: &mut [u8],
        page: u8,
    ) -> Result<u8, ViewError> {
        use lexical_core::{write as itoa, Number};

        match item_n {
            0 => {
                let title_content = pic_str!(b"Type");
                title[..title_content.len()].copy_from_slice(title_content);

                handle_ui_message(self.endorsement_type(), message, page)
            }
            1 => {
                let title_content = pic_str!(b"Branch");
                title[..title_content.len()].copy_from_slice(title_content);

                let mut hex_buf = [0; 32 * 2];
                //this is impossible that will error since the sizes are all checked
                hex::encode_to_slice(&self.branch(), &mut hex_buf).unwrap();

                handle_ui_message(&hex_buf[..], message, page)
            }
            2 => {
                let title_content = pic_str!(b"Blocklevel");
                title[..title_content.len()].copy_from_slice(title_content);

                let mut itoa_buf = [0u8; u32::FORMATTED_SIZE_DECIMAL];

                handle_ui_message(itoa(self.level(), &mut itoa_buf), message, page)
            }
            3 => {
                let title_content = pic_str!(b"ChainID");
                title[..title_content.len()].copy_from_slice(title_content);

                let mut itoa_buf = [0u8; u32::FORMATTED_SIZE_DECIMAL];

                handle_ui_message(itoa(self.chain_id(), &mut itoa_buf), message, page)
            }
            4 if self.is_tenderbake() => {
                let title_content = pic_str!(b"Round");
                title[..title_content.len()].copy_from_slice(title_content);

                let mut itoa_buf = [0u8; u32::FORMATTED_SIZE_DECIMAL];

                handle_ui_message(
                    itoa(self.round().apdu_unwrap(), &mut itoa_buf),
                    message,
                    page,
                )
            }
            _ => Err(ViewError::NoData),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::Preemble;

    const INPUT_HEXES: &[&str] = &["11af1864d90009fedc021ca619c0213f69395e63dea746b6f1ab2c3b68ab5747d50a495c43ae42b1001900000000629f5e5904f4db813cb4c24e7533e853f32dc63554fb059ffc72e896ce8e4376fdb5892354000000210000000102000000040009fedc0000000000000004ffffffff000000040000000061a7fa64128a960b035958da4aebbb71eb0f7d4c79b62817d67388e120fd4f2000e98e05f5956b781ddfb5ae743cfa0d38688af551d39254d3438d1aa8f4f465000000006d9cfa359b2705000000",
                                   "117a06a770002397ee0c68e4ef4a35e2f768012c2f3b560bb80eb8849327696b6843e20842ca5e01345f000000006270c6650469204194e88ec93cdc7ee5d0aa42908ad49e4e0fb7242d406e73d3a48528d4af00000021000000010200000004002397ee0000000000000004ffffffff000000040000000076222f1388f0a7d6b53b96fde78a2b79b50a19282c48eb003cbdffb6b3669e3897440e9db3d73900616e3f720e162588071444071312d967a7d38f7ea23a480c0000000061fed54022b203000000",
                           "11af1864d90009fd6a028bdabe6a6d71140f215e9a5466741aec328fbf8c83a8c346e511d48afd6b837700000000629f46bc0434a0bf047f56a4c10d6a5ab543a117563333e4ba66aecea5d99f977468387a41000000210000000102000000040009fd6a0000000000000004ffffffff00000004000000000c5fd760b3babf2110c085fc07aa5b3fd921f90439f0f1106a2f85ca194ede5934a366913695909603faf23e8856f22703e871df145c3f6e416cf434d5b2f90a000000006d9cfa35504205000000",
                                   "11af1864d90009fd6702a1deb756b7a9d29bf71a2e1eaaba2c8cc9c7c2b7a41ef27d2ceb697ff26e4b4500000000629f468f0401ea87446043f05ef91c0f18902f6cef5760827daeb822f392685a869b70ed17000000210000000102000000040009fd670000000000000004ffffffff00000004000000009ea6ed38e264f9a7d83c4c06904a242a2b5fa8896318e9d0ec07de13c8818dbaeb40ebce04bfefa08a618de10fabe96cca999c720eca9ad0817b2c68f342379f000000006d9cfa3507f202000000",
                                   "11af1864d90009fc75025ac43ace188a2f5c21df248b5bed2a505deec7bd68a84242e75bbb6d41ec7ef100000000629f3730047e22b112420a2b4983d3d394ad324f849bf217441d2c1b912352b361bfa9712b000000210000000102000000040009fc750000000000000004ffffffff000000040000000034b53af6fb6c29b3c6141c3466befc269c7515f80443512be43c7a12476f8f7f12e646513b37f702439d91defa4c1f4bf74b0e39e29b9767af7e00c48c9f726d000000006d9cfa35971402000000",
                                   "11af1864d90009fb2a0203838146d33e3b319e762f0ed3f6336201dab9040f6083b22d536c73772f32f500000000629f21fa04e877ea3d47225895e4652946126b8eb067788477e1f3cad04c299e66cd0c28ec000000210000000102000000040009fb2a0000000000000004ffffffff00000004000000007371860097d16413a83ee78246b5c0f8e14fb59324307e5d79241a19ad04e81035ac9d726404ef5cbcf86a8de00a56ca22c75319fc4c33d23aa23129e83c6a7a000000006d9cfa352c6a05000000",
                                   "11af1864d90009fb0f0232b14954e1e599a3aff048afc777e856ed0201d3a9abf1a1a4482bc6c642d78600000000629f2051044e7b2e973bab73224784532971de2609ed2f7d77ff33fd3083fae956ae8fcf69000000210000000102000000040009fb0f0000000000000004ffffffff000000040000000015b3d6191c6e581dd72d185d7ef50a0d6f9d68a56c7507c8ce1a2c18247b30242613ed2734532c27bff9f005d9565dd71459e4330eeb8f319aca2110d114800e000000006d9cfa35673403000000",
                                   "11af1864d90009faef0201dff343c7fcf125325e3fc3d01071ccfcdb5a56b2281729ed98f620593dbba600000000629f1e7104d3e315d5512d973d581fa4d06f61d2a3f973d57e069c9cb2612aac4a81310373000000210000000102000000040009faef0000000000000004ffffffff00000004000000009eeabb9dac753ef41e5d188b1f9e820259c016049046ced9df994d82c664d09e774476fae2201abaa9056f512ee4bccf82e44ba4c3dea2d4ade7384a27dbee48000000006d9cfa35c70902000000",
                                   "11af1864d90009fa2a0293d1c7d7e9371b23025986174649aeb4839baeaff090e791c087b2c3392d75f600000000629f1192040673f4ba15722a95ba896732ed769a19190e28f5146c49348df0251821bd4f78000000210000000102000000040009fa2a0000000000000004ffffffff0000000400000000b63948c83d8a95ccb3fd8d468edb69ddc639e3897b9104279c7211eefd2763cb86115bb9e7a6ab06f18bc9fb34abee5504e459e0029821ac943e7972334568d2000000006d9cfa3525140f000000",
                                   "11af1864d90009f92202d06f8e677fcd03fb0a517e5e4387fd6762e35edddbffdbdbbe7e44d7823674e900000000629effd604a459958ae43ba40f2af4a9f5ec570c6b61357d82d1b6770e2cf10de8aca20460000000210000000102000000040009f9220000000000000004fffffffe0000000400000001f4796922aeeb2b1481b2313ed9f672f442d58c1c074e576713d5945e687761267b56ec516919893456c609f1df614ce439b33ba91ed66ebf825319fa5f180d35000000016d9cfa352e2502000000",
                                   "11af1864d90009f8e602c7e6dbde632f104a966704f078542c63513000f30d3174fbd394a59fdca4b2f300000000629efbc604f219f6175daf55a19e9e052b7f99c2bf53026549324fd4f16f0fc4dfc1080db6000000250000000102000000040009f8e6000000040000000000000004ffffffff000000040000000166f9c7af926d48754bf1040902726bfbc85ded10becf072df70f5bebbc6ef4d649090a6974de29ca5acc82c6556fb7890e701627f6c13332e4fc3f1521703ce4000000006d9cfa351df100000000",
                                   "11af1864d90009f87202a53d3a1b6612ec373def720371bd69825c9af7ad956419bcfab626f4bb301edf00000000629ef4320434a1c859bb9ba0f2e709147e06e7ab7a6daaa0e814aaf007a486f33f2d7b129d000000210000000102000000040009f8720000000000000004ffffffff0000000400000000fcddcbade65e9667f88ef5ac0b48bdfc5a87c2971f13c532d04f7d9025aea23a6a2566c564d70d02cb924f42e3d1ce54259a2a288e47a8ff2b9689cf24e18c90000000006d9cfa352d9a00000000",
                                   "11af1864d90009f791024e34d00c023d1f1f8e4a8d147a80532aeb7002dd3eb3dcb9f456f9f82ecce38300000000629ee5ff041c061a3aec7bd9353fd8f3071872ba39f54dce886e3e2c2616c0f0c083acba01000000210000000102000000040009f7910000000000000004ffffffff000000040000000054c5ba9c8befa2e558bcf694e9b3b4e601d4158b5d4dd5ab576a4d0f74d4f6d11beddd6a37ae6a2b069e53de6a3d195f1375dec65905d5143e7deffc55561933000000006d9cfa35cc3b00000000",
                                   "11af1864d90009f78e020e8a75b631cf88ea05841d5f0912d71bd71747622233b6fe9134e9c422b3e98300000000629ee5d2043294406c6a7201d90f1fe6936cf21ff31d8939562912b2e556b92bf348d99bec000000210000000102000000040009f78e0000000000000004ffffffff0000000400000000069447f3b38b21a724fcb28247913ed851253ec1600446d1cc8b3bef6b430c7b2687b254010ab9e4359744034d36bfb5ad011e2eb96db2d7331cc0f8f864c517000000006d9cfa35075004000000",
                                   "11af1864d90009f78c026e5d13550f52f41d23f1ed28f8b6f8aa6457169ce0462226a634c49e946f320400000000629ee5b40416a2bf089a311d08573759feae2603127f0a815e3cfda8ebfb930f811fb39b22000000210000000102000000040009f78c0000000000000004ffffffff0000000400000000f04c3584a2c665bba5aa5e766fb8ada6a7d705c4550c6c9c6e6ab336db95b0f7638e3dff40e39ec2703ca4c4ab2325006a819b3d3d5f296b4fa8e2559dff0702000000006d9cfa35110a00000000",
                                   "11af1864d90009f6d802e35df3928869daf47811263b70843d37147498d3ba54df8f88d5d3c07f259c2f00000000629eda4c04895949f7be670eee280b057eb107f4faa0c90bed5c0062a5df621aad864348a3000000210000000102000000040009f6d80000000000000004ffffffff000000040000000010c481fb542824f976a5ef80bb2b4741e659907caf374ce1b72ffe5e8a3c114f71a17e0c6cf2befe635afe5fd6f0d1a8019247124583d5584a22363682586fbf000000006d9cfa356b7e00000000",
                                   "11af1864d90009f5f402648e170d063e51055c858265f829920c01c458f0e8e5fb5c7da94fcb1cf204d600000000629ecbb00439cc7e411f2485d0aebca4f7ca9aac41a2a3ba6abbe4f9eff78c467c9bd7b8c6000000210000000102000000040009f5f40000000000000004ffffffff00000004000000000ac8afead46922bdfc25e3df41d27254325e28b2a599ff8f4205d31ab28f0ec34ffe1a292d45388ab2e628af78ecba84b95a9f0e98e9b0a152091eed399b4c25000000006d9cfa354f5e00000000",
                                   "11af1864d90009f5db0230796b16def6bf8c3ff38e278fe50a1da6b62f17bee12d0c20b01453758a87d700000000629eca25040f25b4dd5568c5a0e9d81ddc511c13b6b0989e3ae12cdbd28e0d46b59d475739000000210000000102000000040009f5db0000000000000004ffffffff0000000400000000266775c80a00d9560ed4f8562f9b2d7b7b7a4abab924b2c8a05ec8ecd465b0741c55dde88bb1e5f3a9da7ade42f0a970da8c0f55d03799fabb3ee25d27c3c1a0000000006d9cfa35c4d900000000",
                                   "11af1864d90009f5be02b7cad5f8aa6d3e3ae8ca47773c3744cbb0506ac22b576ab79ef70b7e491e27ee00000000629ec86d04c79e6d8d9d302d9fcd2c9422e399e7cbd336967ce38c9ed3d271aa2373c3a3ee000000250000000102000000040009f5be000000040000000000000004ffffffff0000000400000001ce25466d4b6e79495b1e6cea0b9ee94d4ae51a8414b0a38adec21d6231fb721709a2438b61dbcfd631c0f00d99f484c4188fd8e5ece197afe8cd9a2f2b817e86000000006d9cfa35343203000000",
                                   "11af1864d90009f56402387e6e7e7a578c017cd6c3478938f9ae6e92069979549eb7bb38fc8eaeb7bdea00000000629ec273042fb762af2514eee32845f09420854859873b598b9f9fb0b26fdd6b9abb875231000000210000000102000000040009f5640000000000000004ffffffff0000000400000000407af996252387773b9bdb26fd058ca511a689285d075b339268b890735fe29d7561a7263a83bcfa9c0081752a59005b95e3ed2b03235c0774e518f3a5ae14d6000000006d9cfa35a0f500000000",
                                   "11af1864d90009f55702c8d92158b4963f720ab60a737e4eb6e7ae52d04e9ddddf387942b4ef8de48f7a00000000629ec1b004e7518c9b89ce381e4f6bbb219ed9c5f60e8f7df514471b0120dbca5e6bc0139f000000210000000102000000040009f5570000000000000004ffffffff000000040000000098cdbe4748f0d1958c001166f3ba851f3b71caffce06721e94a6c8e6b953351ae10248e2a61fc26aed113cd8519e05918c61907221efd94a6e5e91b34678d44e000000006d9cfa35360301000000",
                                   "11af1864d90009f4d902b2e9400938e9d5d0f016577c32095f55f5f4c3dc178d5c6dfe0f15d8ad3c7ddc00000000629eb9d1049320e0c2f27245b81282d993f4d137931df29d05e0f2a1025066bc4dda2450b9000000210000000102000000040009f4d90000000000000004ffffffff0000000400000000830807960115453eb4799654eb2518dc0931a8b335153cb4c1e91379c383094672a9620bda94ab7b4077f3347719c0ab0a3ba17a52d5dd3e930c304ad2f48b8c000000006d9cfa35a8b302000000",
                                   "11af1864d90009f4b202179176de24ccc57d541e6c902a2236903c9e719cb131880e41eaf022bb0b676000000000629eb74c047d553943e2b5a3750ec8bffd9466a908448e59fb8aa359331dffeb2ce1dead73000000210000000102000000040009f4b20000000000000004ffffffff0000000400000000e255a18b75d0135b4e5bd3c8fbe6866bae80b525a47108d385d16a3d8eca2f3098fafa55da2ef29d7ac030d99749178c17c424e5dee9ac505b6343c0424b6b40000000006d9cfa35ea5a04000000",
                                   "11af1864d90009f3ad020465b133840410a7cfb86b22f22cb08e43bd192351f79bf805d292f4d78f3ac300000000629ea6e9047db699b636b9f8b663fe476b4c695fa0e9a1d93d2c35a8312957aed4be4feb85000000210000000102000000040009f3ad0000000000000004ffffffff00000004000000006a7b6a96236561be4c1d0d724e1b12f6ba539d14d42f6966e5f8803a36af4fac432f7a69c1ca24d848f46fea1e5bf3077d2a3706f8de3902496cd383f9393fca000000006d9cfa352c9d03000000",
                                   "11af1864d90009f35002888b0742d241f94f6a2a85e660c203612041da44213df6aeca30121df913095f00000000629ea12604b52338eea7b7092b0d19b24f027b40d3e3d1c4f671afdaf2512cc0e2e51492f2000000210000000102000000040009f3500000000000000004ffffffff00000004000000007d3a466706e81c0a01f8406d2d3c60cd28c1416b5c5f139a8e2546b357994a81980785899f11c9235ad67b60cf2b043baed4bd2284c94302759c98d99ae67439000000006d9cfa3581a300000000",
                                   "11af1864d90009f2820204dd7a96959501a7d4167fabf4b23a2eae395d9898a46341744627c2a839e93b00000000629e93a204ecf40d09d731694def5a803a67393ff5303fee17e06182b2c5c96cf5535adba9000000210000000102000000040009f2820000000000000004ffffffff00000004000000002cdbe8086c6754abc53ba6d91a60c5e9ca4294497c1c98b9024238f5e329d85b695fd5894820b5d33d604cb34065d992e108421b873fa29eee562d5994d9482a000000006d9cfa357fcd03000000",
                                   "11af1864d90009f240024ddef3f1ac3b6ee71c111360c1de0886d5bbc817104862b1f8b611189fb2200800000000629e8fc4048ef25a0b6ab038dfea3480ff6f0bf129e28d090b888cec083f111da5a975371b000000210000000102000000040009f2400000000000000004ffffffff00000004000000008158bb5b38b3489a4d3fbd480b0e2be5b195d9deb63879191fa3aa5f8f26ea286b80303fe3c39f170c808e4b3fcddafbe9fa46d356d282f9549d4f8617405842000000006d9cfa35a8cc0000ff31209e63118a28542d06d6d3c2306898eaee19beff642aca5cdfefc76785f41800",
                                   "11af1864d90009f0bd026959b7c513578ad787c74562caa96cb489b6116892e3d8cac95f806eebbde1ca00000000629e5d2e048d22423ba39f965cc70e15c2b2c4294175f05a27c4a7c2dbcb59e4d0293c911e000000210000000102000000040009f0bd0000000000000004ffffffff00000004000000005e2555e7965f90c129456f6c0917208a5e7e496d962363444a625d68e0dda48142226d7fa90d82f93bdd22597d941f9bb0be4e7595bceec223216ab3981a2add000000006d9cfa35e7c703000000",
                                   "11af1864d90009f0ba026d81c9b842f5826d6956cd2e9fdb5ed7111a2420b08b09db29493e66217c058e00000000629e5d0104dd49102461ad1ded6a25df8c4d0b75ee5e31119e9a241e8de412f45611877782000000210000000102000000040009f0ba0000000000000004ffffffff0000000400000000ce73c4fe2577e58611f1aea2417e3f48659c029482ce9c44bb561baa02cb6eb6258ff90c4b760e4a651873a91c141b7246aff12431889bb1455a5c5a01fcbb28000000006d9cfa358d6801000000",
                                   "11af1864d90009f06b02c9b0e0671c5a06ce3fc8b3a47fa8dc537a8fccf1f9490947686a1e7b606b645e00000000629e581004dcf916e1dc1ccfbd47700b18ed046a20cd9efbd7fbef74b271c8f90b5f0d0b0d000000210000000102000000040009f06b0000000000000004ffffffff0000000400000000bc31476b5c4db4cc425b8a54e9293d560268ffb81bbb4f948a5b92942381bd9f680ea22a191661013379a541f192c32e6c17c9167afa9fd179173104588f1bd4000000006d9cfa3506a301000000",
                                   "11af1864d90009efa902ff76932790131e38660c768885bda62a6874018c6210cf7f8ab3007cb90f2c4700000000629e4b8604e8b4c9242f18a20c535752bb0747c6f930c789b668b3af1d9fc389b0339f92cf000000210000000102000000040009efa90000000000000004ffffffff0000000400000000fc141f5c4c741106472abda625ce1e55b1df37e5caed84039378ab39c207ab46b708368e743eaa3b13315e9b494d62dd3b9f2e87a0573777ce27e8bf0d85db30000000006d9cfa352cad01000000",
                                   "11af1864d90009eea5028d2a9ada9ef878c3ff70e022a1f9b7450e9cdc72ff8b869a4d39b8bf5f607cad00000000629e3ace0460f87e12f1f50ba6fd6ad9c2c429321d50cb4ea01e0c9bac37ffe950513ee55e000000210000000102000000040009eea50000000000000004ffffffff0000000400000000c1d7e0f818404d41980b7f27c610d54ec9d88ee8f967ac8b1fe5de1ec034f39c8044332d4a69338fb36d5322a66842d33b4a58f79b2316c88f19815a1e906538000000006d9cfa3537f210000000",
                                   "11af1864d90009ee890250895ce5dcceb62d075cd8237bc192837ec09a4ec0203148adcc9fb6a761574b00000000629e391104dc158a81068ab5857aa3f5f79c0fa89fc62753310b2a1eb087d654db8f6daa30000000250000000102000000040009ee89000000040000000000000004ffffffff0000000400000001a30e74129ad796253a5e13a78ce980d06cfa0ef718766e6a5c2cab0d7fe5b7fd9885cffdeaf3a6c261565a98753550d120a5487837434c278ed2b4fc40077f9f000000006d9cfa355af703000000",
                                   "11af1864d90009ee8202f14429fd99591672e68da3e563ca6868d5bc4de301cc62c76320389faf92a87a00000000629e388504132a21965deff49b326f1841e104cb1f4b76c2621653d93d4a7711b699fec71e000000210000000102000000040009ee820000000000000004fffffffe0000000400000000e540b30f0efb5a56e861853cb94b45a289b96a1d97f331c71e5f91d0f7931c25954d6d22eeada5d3e68a165b9be48b1d1b1847a909d792dbd7aaeef175ab90a1000000006d9cfa358a4701000000",
                                   "11af1864d90009ed750245315eb17a79adf61401cc0cafc6cfd5afa9fe27e4aec05b139644ed220498db00000000629e27690404430ce0bad53f49746a17a3f6589c6720c8827f858b617cd53917b8c8b90921000000210000000102000000040009ed750000000000000004ffffffff0000000400000000a00d907e54fd62d319b5d0f3fb967b8a6bcb7baf967d4fc258be681fb5bd8054d9f85d05b4e1d92d249642ad812cdce70f218c7e2b5090b06d5aeddb09485142000000006d9cfa35c5a405000000",
                                   "11af1864d90009ecf40252d8646963eb89eb361d6efc7c65944bc1349e3a3d0893d84e9ab66417e6973200000000629e1f26046f84e656ed46215f5f0d0ba33d7398699a32d0606ca0eed57060ce805de6925f000000210000000102000000040009ecf40000000000000004ffffffff0000000400000000af98c47fa7318c0f3586c852af26bc5144a32daa3789beeedcfde35d9036c9b863c368222910f17e44c11ab77bd10e003e7462c15b5d59571d8d8428885bff37000000006d9cfa358d0406000000",
                                   "11af1864d90009ecbc02799fb65a04c14798a69d2e1c1b961e0523984c200b9fd0857fe49bf7b0d76aa600000000629e1b9d0441fbe6aa201798312c624bf96472a5bf6c7c41cc1c645fbca00a0538515d42a6000000250000000102000000040009ecbc000000040000000000000004ffffffff0000000400000001b76969cc81221cacf2dd947965b91c5d05da0dd97237d4a2d923bf5f12db32ed949d0274c0c6aa5b07ad3b1bc6199886e9e425ff0aa580c265dcb5d7c04e1eb7000000006d9cfa356c2803000000",
                           "11af1864d90009ecab027bc2650d8cbf622a75a9fa50dde7e4a6102cba8de75093031c0e0b24df7c8c3400000000629e1a5d04c7256543be543c4615e1e1a88d7172d23f7d6948243389cc2b3a469c15b4e0d5000000210000000102000000040009ecab0000000000000004ffffffff00000004000000020ca4b32297581fb27a602a6b5368df9e7354a5e72e86aabd5f459ee5c21314111de748b18728600ee29afa5d92160cadfef7a4edda0e012ba0052d10cbc42dfb000000026d9cfa358ea500000000"];

    #[test]
    fn misc_tenderbake_block_blobs() {
        for (i, hex) in INPUT_HEXES.iter().enumerate() {
            let input = hex::decode(hex).expect("invalid input hex");
            let (rem, preemble) = Preemble::from_bytes(&input)
                .unwrap_or_else(|e| panic!("couldn't parse preemble: {:?}, idx #{}", e, i));

            assert_eq!(preemble, Preemble::TenderbakeBlock);

            let (_, blockdata) = BlockData::from_bytes(rem)
                .unwrap_or_else(|e| panic!("unable to parse blockdata: {:?}, idx #{}", e, i));
            std::dbg!(&blockdata);
            assert!(matches!(blockdata.fitness, Fitness::Tenderbake(_)));
        }
    }
}
