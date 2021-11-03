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
//! Module containing all crypto primitives and utilities
//! for rust ledger apps

use std::convert::TryFrom;

pub use bolos_common::bip32;

//Constants
use crate::raw::{
    cx_curve_e_CX_CURVE_BLS12_381_G1, cx_curve_e_CX_CURVE_BrainPoolP256R1,
    cx_curve_e_CX_CURVE_BrainPoolP256T1, cx_curve_e_CX_CURVE_BrainPoolP320R1,
    cx_curve_e_CX_CURVE_BrainPoolP320T1, cx_curve_e_CX_CURVE_BrainPoolP384R1,
    cx_curve_e_CX_CURVE_BrainPoolP384T1, cx_curve_e_CX_CURVE_BrainPoolP512R1,
    cx_curve_e_CX_CURVE_BrainPoolP512T1, cx_curve_e_CX_CURVE_Curve25519,
    cx_curve_e_CX_CURVE_Curve448, cx_curve_e_CX_CURVE_Ed25519, cx_curve_e_CX_CURVE_Ed448,
    cx_curve_e_CX_CURVE_FRP256V1, cx_curve_e_CX_CURVE_NONE, cx_curve_e_CX_CURVE_SECP256K1,
    cx_curve_e_CX_CURVE_SECP256R1, cx_curve_e_CX_CURVE_SECP384R1, cx_curve_e_CX_CURVE_SECP521R1,
    cx_curve_e_CX_CURVE_Stark256,
};

#[derive(Clone, Copy)]
pub enum Curve {
    None,

    /* Secp.org */
    Secp256K1,
    Secp256R1,
    Secp384R1,
    Secp521R1,

    /* Brainpool */
    BrainPoolP256T1,
    BrainPoolP256R1,
    BrainPoolP320T1,
    BrainPoolP320R1,
    BrainPoolP384T1,
    BrainPoolP384R1,
    BrainPoolP512T1,
    BrainPoolP512R1,

    /* NIST P256 */
    Nistp256, //alias to Secp256R1
    Nistp384, //alias to Secp384R1
    Nistp521, //alias to Secp521R1

    /* ANSSI P256 */
    Frp256V1,

    /* Stark */
    Stark256,

    /* BLS */
    Bls12_381G1,

    /* Ed25519 */
    Ed25519,
    Ed448,

    /* Curve25519 */
    Curve25519,
    Curve448,
}

impl TryFrom<u8> for Curve {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value as u32 {
            cx_curve_e_CX_CURVE_NONE => Ok(Self::None),
            cx_curve_e_CX_CURVE_SECP256K1 => Ok(Self::Secp256K1),
            cx_curve_e_CX_CURVE_SECP256R1 => Ok(Self::Secp256R1),
            cx_curve_e_CX_CURVE_SECP384R1 => Ok(Self::Secp384R1),
            cx_curve_e_CX_CURVE_SECP521R1 => Ok(Self::Secp521R1),
            cx_curve_e_CX_CURVE_BrainPoolP256T1 => Ok(Self::BrainPoolP256T1),
            cx_curve_e_CX_CURVE_BrainPoolP256R1 => Ok(Self::BrainPoolP256R1),
            cx_curve_e_CX_CURVE_BrainPoolP320T1 => Ok(Self::BrainPoolP320T1),
            cx_curve_e_CX_CURVE_BrainPoolP320R1 => Ok(Self::BrainPoolP320R1),
            cx_curve_e_CX_CURVE_BrainPoolP384T1 => Ok(Self::BrainPoolP384T1),
            cx_curve_e_CX_CURVE_BrainPoolP384R1 => Ok(Self::BrainPoolP384R1),
            cx_curve_e_CX_CURVE_BrainPoolP512T1 => Ok(Self::BrainPoolP512T1),
            cx_curve_e_CX_CURVE_BrainPoolP512R1 => Ok(Self::BrainPoolP512R1),
            cx_curve_e_CX_CURVE_FRP256V1 => Ok(Self::Frp256V1),
            cx_curve_e_CX_CURVE_Stark256 => Ok(Self::Stark256),
            cx_curve_e_CX_CURVE_BLS12_381_G1 => Ok(Self::Bls12_381G1),

            cx_curve_e_CX_CURVE_Ed25519 => Ok(Self::Ed25519),
            cx_curve_e_CX_CURVE_Ed448 => Ok(Self::Ed448),

            cx_curve_e_CX_CURVE_Curve25519 => Ok(Self::Curve25519),
            cx_curve_e_CX_CURVE_Curve448 => Ok(Self::Curve448),
            _ => Err(()),
        }
    }
}

impl Into<u8> for Curve {
    fn into(self) -> u8 {
        let n = match self {
            Curve::None => cx_curve_e_CX_CURVE_NONE,
            Curve::Secp256K1 => cx_curve_e_CX_CURVE_SECP256K1,
            Curve::Secp256R1 | Curve::Nistp256 => cx_curve_e_CX_CURVE_SECP256R1,
            Curve::Secp384R1 | Curve::Nistp384 => cx_curve_e_CX_CURVE_SECP384R1,
            Curve::Secp521R1 | Curve::Nistp521 => cx_curve_e_CX_CURVE_SECP521R1,
            Curve::BrainPoolP256T1 => cx_curve_e_CX_CURVE_BrainPoolP256T1,
            Curve::BrainPoolP256R1 => cx_curve_e_CX_CURVE_BrainPoolP256R1,
            Curve::BrainPoolP320T1 => cx_curve_e_CX_CURVE_BrainPoolP320T1,
            Curve::BrainPoolP320R1 => cx_curve_e_CX_CURVE_BrainPoolP320R1,
            Curve::BrainPoolP384T1 => cx_curve_e_CX_CURVE_BrainPoolP384T1,
            Curve::BrainPoolP384R1 => cx_curve_e_CX_CURVE_BrainPoolP384R1,
            Curve::BrainPoolP512T1 => cx_curve_e_CX_CURVE_BrainPoolP512T1,
            Curve::BrainPoolP512R1 => cx_curve_e_CX_CURVE_BrainPoolP512R1,
            Curve::Frp256V1 => cx_curve_e_CX_CURVE_FRP256V1,
            Curve::Stark256 => cx_curve_e_CX_CURVE_Stark256,
            Curve::Bls12_381G1 => cx_curve_e_CX_CURVE_BLS12_381_G1,
            Curve::Ed25519 => cx_curve_e_CX_CURVE_Ed25519,
            Curve::Ed448 => cx_curve_e_CX_CURVE_Ed448,
            Curve::Curve25519 => cx_curve_e_CX_CURVE_Curve25519,
            Curve::Curve448 => cx_curve_e_CX_CURVE_Ed448,
        };
        n as u8
    }
}

impl Curve {
    pub fn is_weirstrass(&self) -> bool {
        match self {
            Self::Secp256K1
            | Self::Secp256R1
            | Self::Secp384R1
            | Self::Secp521R1
            | Self::BrainPoolP256T1
            | Self::BrainPoolP256R1
            | Self::BrainPoolP320T1
            | Self::BrainPoolP320R1
            | Self::BrainPoolP384T1
            | Self::BrainPoolP384R1
            | Self::BrainPoolP512T1
            | Self::BrainPoolP512R1
            | Self::Nistp256
            | Self::Nistp384
            | Self::Nistp521
            | Self::Frp256V1
            | Self::Stark256
            | Self::Bls12_381G1 => true,
            _ => false,
        }
    }

    pub fn is_twisted_edward(&self) -> bool {
        match self {
            Self::Ed25519 | Self::Ed448 => true,
            _ => false,
        }
    }

    pub fn is_montgomery(&self) -> bool {
        match self {
            Self::Curve25519 | Self::Curve448 => true,
            _ => false,
        }
    }

    pub fn domain_length(&self) -> Option<usize> {
        match self {
            Curve::None => None,
            Curve::Secp256K1 | Curve::Secp256R1 | Curve::Nistp256 => Some(32),
            Curve::Secp384R1 | Curve::Nistp384 => Some(48),
            Curve::Secp521R1 | Curve::Nistp521 => Some(66),
            Curve::BrainPoolP256T1 | Curve::BrainPoolP256R1 => Some(32),
            Curve::BrainPoolP320T1 | Curve::BrainPoolP320R1 => Some(40),
            Curve::BrainPoolP384T1 | Curve::BrainPoolP384R1 => Some(48),
            Curve::BrainPoolP512T1 | Curve::BrainPoolP512R1 => Some(64),
            Curve::Frp256V1 => Some(32),
            Curve::Stark256 => None,
            Curve::Bls12_381G1 => None,
            Curve::Ed25519 => Some(32),
            Curve::Ed448 => Some(57),
            Curve::Curve25519 => Some(32),
            Curve::Curve448 => Some(56),
        }
    }
}

#[cfg(nanos)]
use crate::raw::HDW_SLIP21;
use crate::raw::{HDW_ED25519_SLIP10, HDW_NORMAL};

#[derive(Clone, Copy)]
pub enum Mode {
    BIP32,
    Ed25519Slip10,
    #[cfg(nanos)]
    Slip21,
}

impl TryFrom<u8> for Mode {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value as u32 {
            HDW_NORMAL => Ok(Self::BIP32),
            HDW_ED25519_SLIP10 => Ok(Self::Ed25519Slip10),
            #[cfg(nanos)]
            HDW_SLIP21 => Ok(Self::Slip21),
            _ => Err(()),
        }
    }
}

impl Into<u8> for Mode {
    fn into(self) -> u8 {
        let n = match self {
            Mode::BIP32 => HDW_NORMAL,
            Mode::Ed25519Slip10 => HDW_ED25519_SLIP10,
            #[cfg(nanos)]
            Mode::Slip21 => HDW_SLIP21,
        };

        n as u8
    }
}

impl Default for Mode {
    fn default() -> Self {
        Self::BIP32
    }
}

mod bindings {
    use super::{bip32::BIP32Path, Curve, Mode};
    use crate::errors::{catch, Error};

    pub fn os_perso_derive_node_with_seed_key<const B: usize>(
        mode: Mode,
        curve: Curve,
        path: &BIP32Path<B>,
        out: &mut [u8; 64],
    ) -> Result<(), Error> {
        zemu_sys::zemu_log_stack("os_perso_derive_node_with_seed_key\x00");
        let curve: u8 = curve.into();
        let mode: u8 = mode.into();

        let out_p = out.as_mut().as_mut_ptr();
        let (components, path_len) = {
            let components = path.components();
            (components.as_ptr(), components.len() as u32)
        };

        cfg_if! {
            if #[cfg(nanox)] {
                let might_throw = || unsafe {
                    crate::raw::os_perso_derive_node_bip32_seed_key(
                        mode as _,
                        curve as _,
                        components as *const _,
                        path_len as _,
                        out_p as *mut _,
                        std::ptr::null_mut(),
                        std::ptr::null_mut(),
                        0
                    );
                };

                catch(might_throw)?;
            } else if #[cfg(nanos)] {
                let might_throw = || unsafe {
                    crate::raw::os_perso_derive_node_with_seed_key(
                        mode as _,
                        curve as _,
                        components as *const _,
                        path_len as _,
                        out_p as *mut _,
                        std::ptr::null_mut(),
                        std::ptr::null_mut(),
                        0
                    )
                };

                catch(might_throw)?;
            } else {
                unimplemented!("os derive called in non-bolos")
            }
        }

        Ok(())
    }
}

pub mod ecfp256;
