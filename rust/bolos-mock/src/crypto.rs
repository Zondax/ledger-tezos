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
pub use bolos_common::bip32;

use std::convert::TryFrom;

#[derive(Clone, Copy)]
pub enum Curve {
    Secp256K1,
    Secp256R1,

    Ed25519,
}

impl TryFrom<u8> for Curve {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value as u32 {
            1 => Ok(Self::Secp256K1),
            2 => Ok(Self::Secp256R1),
            3 => Ok(Self::Ed25519),

            _ => Err(()),
        }
    }
}

impl From<Curve> for u8 {
    fn from(from: Curve) -> Self {
        match from {
            Curve::Secp256K1 => 1,
            Curve::Secp256R1 => 2,
            Curve::Ed25519 => 3,
        }
    }
}

impl Curve {
    pub fn is_weirstrass(&self) -> bool {
        matches!(self, Self::Secp256K1 | Self::Secp256R1)
    }

    pub fn is_twisted_edward(&self) -> bool {
        matches!(self, Self::Ed25519)
    }

    pub fn is_montgomery(&self) -> bool {
        false
    }
}

#[derive(Clone, Copy)]
pub enum Mode {
    BIP32,
    Ed25519Slip10,
    // Slip21,
}

impl TryFrom<u8> for Mode {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value as u32 {
            0 => Ok(Self::BIP32),
            1 => Ok(Self::Ed25519Slip10),
            // 2 => Ok(Self::Slip21),
            _ => Err(()),
        }
    }
}

impl From<Mode> for u8 {
    fn from(from: Mode) -> Self {
        match from {
            Mode::BIP32 => 0,
            Mode::Ed25519Slip10 => 1,
            // Mode::Slip21 => HDW_SLIP21,
        }
    }
}

impl Default for Mode {
    fn default() -> Self {
        Self::BIP32
    }
}

pub mod ecfp256;
