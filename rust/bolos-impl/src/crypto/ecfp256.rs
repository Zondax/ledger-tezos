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
use zeroize::{Zeroize, Zeroizing};

use super::{bip32::BIP32Path, Curve, Mode};
use crate::{errors::Error, hash::HasherId, raw::cx_ecfp_private_key_t};

#[derive(Clone, Copy)]
pub struct PublicKey {
    curve: Curve,
    len: usize,
    w: [u8; 65],
}

impl PublicKey {
    pub fn compress(&mut self) -> Result<(), Error> {
        match self.curve {
            Curve::Ed25519 => {
                let comp_len = cx_edward_compress_point(self.curve, &mut self.w[..])?;
                self.len = comp_len;

                Ok(())
            }
            _ => Ok(()),
        }
    }

    pub fn curve(&self) -> Curve {
        self.curve
    }

    pub fn len(&self) -> usize {
        self.len
    }
}

impl AsRef<[u8]> for PublicKey {
    fn as_ref(&self) -> &[u8] {
        &self.w[..self.len]
    }
}

pub struct SecretKey<const B: usize> {
    mode: Mode,
    curve: Curve,
    path: BIP32Path<B>,
}

impl<const B: usize> SecretKey<B> {
    pub const fn new(mode: Mode, curve: Curve, path: BIP32Path<B>) -> Self {
        Self { mode, curve, path }
    }

    pub const fn curve(&self) -> Curve {
        self.curve
    }

    fn generate(&self) -> Result<Zeroizing<cx_ecfp_private_key_t>, Error> {
        // Prepare secret key data with the ledger's key
        let mut sk_data =
            super::bindings::os_perso_derive_node_with_seed_key(self.mode, self.curve, &self.path)?;

        // Use the secret key data to prepare a secret key
        let sk_r = cx_ecfp_init_private_key(self.curve, Some(&sk_data[..]));
        // let's zeroize the sk_data right away before we return
        sk_data.zeroize();

        //map secret so Zeroizing to make sure it's zeroed out on drop
        sk_r.map(Zeroizing::new)
    }

    pub fn public(&self) -> Result<PublicKey, Error> {
        //get keypair with the generated secret key
        // discard secret key as it's not necessary anymore
        let (_, pk) = cx_ecfp_generate_pair(Some(self), self.curve)?;

        Ok(PublicKey {
            curve: self.curve,
            len: pk.W_len as usize,
            w: pk.W,
        })
    }

    #[inline(never)]
    pub fn sign<H>(&self, data: &[u8], out: &mut [u8]) -> Result<usize, Error>
    where
        H: HasherId,
        H::Id: Into<u8>,
    {
        let crv = self.curve;
        if crv.is_weirstrass() {
            let (parity, size) = bindings::cx_ecdsa_sign::<H, B>(self, data, out)?;
            //FIXME: if this is part of generic crate, this should not be here as it is non-standard!
            if parity {
                out[0] |= 0x01;
            }

            Ok(size)
        } else if crv.is_twisted_edward() {
            bindings::cx_eddsa_sign(self, data, out)
        } else if crv.is_montgomery() {
            todo!("montgomery sign")
        } else {
            todo!("unknown signature type")
        }
    }
}

mod bindings {
    use super::{Curve, Error, HasherId, SecretKey};
    use crate::{
        errors::catch,
        raw::{cx_ecfp_private_key_t, cx_ecfp_public_key_t},
    };
    use zeroize::{Zeroize, Zeroizing};

    pub fn cx_edward_compress_point(curve: Curve, p: &mut [u8]) -> Result<usize, Error> {
        let curve: u8 = curve.into();

        cfg_if! {
            if #[cfg(nanox)] {
                let might_throw = || unsafe {
                    crate::raw::cx_edward_compress_point(
                        curve as _,
                        p.as_mut_ptr() as *mut _,
                        p.len() as u32 as _,
                    );
                };

                catch(might_throw)?;
                Ok(33)
            } else if #[cfg(nanos)] {
                match unsafe { crate::raw::cx_edwards_compress_point_no_throw(
                    curve as _,
                    p.as_mut_ptr() as *mut _,
                    p.len() as u32 as _
                )} {
                    0 => Ok(33),
                    err => Err(err.into())
                }
            } else {
                todo!("edwards_compress_point called in non-bolos");
            }
        }
    }

    pub fn cx_ecfp_init_private_key(
        curve: Curve,
        sk_data: Option<&[u8]>,
    ) -> Result<cx_ecfp_private_key_t, Error> {
        let curve: u8 = curve.into();

        let sk_data: *const u8 = match sk_data {
            None => std::ptr::null(),
            Some(data) => data.as_ptr(),
        };

        let mut out = cx_ecfp_private_key_t::default();

        cfg_if! {
            if #[cfg(nanox)] {
                let might_throw = || unsafe {
                    crate::raw::cx_ecfp_init_private_key(
                        curve as _,
                        sk_data as *const _,
                        32 as _,
                        &mut out as *mut _,
                    );
                };

                catch(might_throw)?;
            } else if #[cfg(nanos)] {
                match unsafe { crate::raw::cx_ecfp_init_private_key_no_throw(
                    curve as _,
                    sk_data as *const _,
                    32 as _,
                    &mut out as *mut _,
                )} {
                    0 => {},
                    err => return Err(err.into()),
                }
            } else {
                todo!("init ecfp_private_key called in non-bolos");
            }
        }

        Ok(out)
    }

    pub fn cx_ecfp_generate_pair<const B: usize>(
        sk: Option<&SecretKey<B>>,
        curve: Curve,
    ) -> Result<(Zeroizing<cx_ecfp_private_key_t>, cx_ecfp_public_key_t), Error> {
        let curve: u8 = curve.into();

        let (mut sk, keep) = match sk {
            Some(sk) => (sk.generate()?, true),
            None => (Zeroizing::new(Default::default()), false),
        };
        let mut pk = cx_ecfp_public_key_t::default();
        let sk_mut_ref: &mut cx_ecfp_private_key_t = &mut sk;

        cfg_if! {
            if #[cfg(nanox)] {
                let might_throw = || unsafe {
                    crate::raw::cx_ecfp_generate_pair(
                        curve as _,
                        &mut pk as *mut _,
                        sk_mut_ref as *mut _,
                        keep as u8 as _,
                    );
                };

                if let Err(e) = catch(might_throw) {
                    sk.zeroize();
                    return Err(e.into());
                }
            } else if #[cfg(nanos)] {
                match unsafe { crate::raw::cx_ecfp_generate_pair_no_throw(
                    curve as _,
                    &mut pk as *mut _,
                    sk_mut_ref as *mut _,
                    keep,
                )} {
                    0 => (),
                    err => {
                        sk.zeroize();
                        return Err(err.into())
                    },
                }
            } else {
                todo!("generate_ecfp_keypair called in non-bolos");
            }
        }

        Ok((sk, pk))
    }

    //first item says if Y is odd when computing k.G
    // second item in the tuple is the number of bytes written to `sig_out`
    pub fn cx_ecdsa_sign<H, const B: usize>(
        sk: &SecretKey<B>,
        data: &[u8],
        sig_out: &mut [u8],
    ) -> Result<(bool, usize), Error>
    where
        H: HasherId,
        H::Id: Into<u8>,
    {
        use crate::raw::CX_RND_RFC6979;

        let id: u8 = H::id().into();

        let crv = sk.curve;

        let mut raw_sk = sk.generate()?;
        let raw_sk: &mut cx_ecfp_private_key_t = &mut raw_sk;
        let raw_sk = raw_sk as *const _;

        let (data, data_len) = (data.as_ptr(), data.len() as u32);
        let sig = sig_out.as_mut_ptr();

        let mut sig_len = match crv.domain_length() {
            Some(n) => 6 + 2 * (n + 1),
            None => sig_out.len(),
        } as u32;

        let mut info = 0;

        cfg_if! {
            if #[cfg(nanox)] {
                let might_throw = || unsafe { crate::raw::cx_ecdsa_sign(
                    raw_sk,
                    CX_RND_RFC6979 as _,
                    id as _,
                    data,
                    data_len as _,
                    sig,
                    sig_len as _,
                    &mut info as *mut u32 as *mut _,
                )};

                sig_len = catch(might_throw)? as u32;
            } else if #[cfg(nanos)] {
                match unsafe { crate::raw::cx_ecdsa_sign_no_throw(
                    raw_sk,
                    CX_RND_RFC6979,
                    id as _,
                    data,
                    data_len as _,
                    sig,
                    &mut sig_len as *mut _,
                    &mut info as *mut u32 as *mut _,
                )} {
                    0 => {},
                    err => return Err(err.into()),
                }
            } else {
                todo!("cx_ecdsa_sign called in not bolos")
            }
        }

        Ok((info == crate::raw::CX_ECCINFO_PARITY_ODD, sig_len as usize))
    }

    pub fn cx_eddsa_sign<const B: usize>(
        sk: &SecretKey<B>,
        data: &[u8],
        sig_out: &mut [u8],
    ) -> Result<usize, Error> {
        let id: u8 = crate::hash::Sha512::id().into();

        let crv = sk.curve;

        let mut raw_sk = sk.generate()?;
        let raw_sk: &mut cx_ecfp_private_key_t = &mut raw_sk;
        let raw_sk = raw_sk as *const _;

        let (data, data_len) = (data.as_ptr(), data.len() as u32);
        let sig = sig_out.as_mut_ptr();

        let mut sig_len = match crv.domain_length() {
            Some(n) => 6 + 2 * (n + 1),
            None => sig_out.len(),
        } as u32;

        cfg_if! {
            if #[cfg(nanox)] {
                let might_throw = || unsafe { crate::raw::cx_eddsa_sign(
                    raw_sk,
                    0 as _,
                    id as _,
                    data,
                    data_len as _,
                    std::ptr::null(),
                    0,
                    sig,
                    sig_len as _,
                    std::ptr::null_mut(),
                )};

                sig_len = catch(might_throw)? as u32;
            } else if #[cfg(nanos)] {
                match unsafe { crate::raw::cx_eddsa_sign_no_throw(
                    raw_sk,
                    id as _,
                    data,
                    data_len as _,
                    sig,
                    sig_len as _,
                )} {
                    0 => {
                        let crv: u8 = crv.into();
                        match unsafe { crate::raw::cx_ecdomain_parameters_length(
                            crv as _,
                            &mut sig_len as *mut _
                        )} {
                            0 => {sig_len *= 2;},
                            err => return Err(err.into()),
                        }
                    },
                    err => return Err(err.into()),
                }
            } else {
                todo!("cx_eddsa_sign called in not bolos")
            }
        }

        Ok(sig_len as usize)
    }
}
use bindings::*;
