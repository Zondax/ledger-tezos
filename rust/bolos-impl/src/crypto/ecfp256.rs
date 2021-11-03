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

use core::{mem::MaybeUninit, ptr::addr_of_mut};

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

    #[inline(never)]
    fn generate(&self) -> Result<Zeroizing<cx_ecfp_private_key_t>, Error> {
        let mut out = MaybeUninit::uninit();

        self.generate_into(&mut out)?;

        Ok(Zeroizing::new(unsafe { out.assume_init() }))
    }

    fn generate_into(&self, out: &mut MaybeUninit<cx_ecfp_private_key_t>) -> Result<(), Error> {
        zemu_sys::zemu_log_stack("SecretKey::generate_into\x00");
        // Prepare secret key data with the ledger's key
        let mut sk_data = [0; 64];

        super::bindings::os_perso_derive_node_with_seed_key(
            self.mode,
            self.curve,
            &self.path,
            &mut sk_data,
        )?;

        // Use the secret key data to prepare a secret key
        let sk_r = cx_ecfp_init_private_key_into(self.curve, Some(&sk_data[..]), out);
        // let's zeroize the sk_data right away before we return
        sk_data.zeroize();

        sk_r
    }

    #[inline(never)]
    pub fn public(&self) -> Result<PublicKey, Error> {
        let mut out = MaybeUninit::uninit();

        self.public_into(&mut out)?;

        //this is safe as the call above initialized it
        Ok(unsafe { out.assume_init() })
    }

    #[inline(never)]
    pub fn public_into(&self, out: &mut MaybeUninit<PublicKey>) -> Result<(), Error> {
        zemu_sys::zemu_log_stack("SecretKey::public_into\x00");
        //get keypair with the generated secret key
        // discard secret key as it's not necessary anymore
        let (_, pk) = cx_ecfp_generate_pair(Some(self), self.curve)?;

        let out = out.as_mut_ptr();
        //the ptr is good and there's no uninit reads
        unsafe {
            addr_of_mut!((*out).curve).write(self.curve);
            addr_of_mut!((*out).len).write(pk.W_len as usize);
            addr_of_mut!((*out).w).write(pk.W);
        }

        Ok(())
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
    #![allow(unused_imports)]

    use super::{Curve, Error, HasherId, SecretKey};
    use crate::{
        errors::catch,
        raw::{cx_ecfp_private_key_t, cx_ecfp_public_key_t},
    };
    use core::mem::MaybeUninit;
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
                unimplemented!("edwards_compress_point called in non-bolos");
            }
        }
    }

    #[allow(dead_code)]
    pub fn cx_ecfp_init_private_key(
        curve: Curve,
        sk_data: Option<&[u8]>,
    ) -> Result<cx_ecfp_private_key_t, Error> {
        let mut out = MaybeUninit::uninit();
        cx_ecfp_init_private_key_into(curve, sk_data, &mut out)?;

        //this is safe because the data is now initialized
        Ok(unsafe { out.assume_init() })
    }

    pub fn cx_ecfp_init_private_key_into(
        curve: Curve,
        sk_data: Option<&[u8]>,
        out: &mut MaybeUninit<cx_ecfp_private_key_t>,
    ) -> Result<(), Error> {
        zemu_sys::zemu_log_stack("cx_ecfp_init_private_key_into\x00");
        let curve: u8 = curve.into();

        let sk_data: *const u8 = match sk_data {
            None => std::ptr::null(),
            Some(data) => data.as_ptr(),
        };

        let out = out.as_mut_ptr();

        cfg_if! {
            if #[cfg(nanox)] {
                let might_throw = || unsafe {
                    crate::raw::cx_ecfp_init_private_key(
                        curve as _,
                        sk_data as *const _,
                        32 as _,
                        out,
                    );
                };

                catch(might_throw)?;
            } else if #[cfg(nanos)] {
                match unsafe { crate::raw::cx_ecfp_init_private_key_no_throw(
                    curve as _,
                    sk_data as *const _,
                    32 as _,
                    out,
                )} {
                    0 => {},
                    err => return Err(err.into()),
                }
            } else {
                unimplemented!("init ecfp_private_key called in non-bolos");
            }
        }

        Ok(())
    }

    pub fn cx_ecfp_generate_pair<const B: usize>(
        sk: Option<&SecretKey<B>>,
        curve: Curve,
    ) -> Result<(Zeroizing<cx_ecfp_private_key_t>, cx_ecfp_public_key_t), Error> {
        zemu_sys::zemu_log_stack("cx_ecfp_generate_pair\x00");
        let curve: u8 = curve.into();

        let mut raw_sk = MaybeUninit::zeroed();
        let mut pk = MaybeUninit::zeroed();

        let keep = match sk {
            Some(sk) => {
                sk.generate_into(&mut raw_sk)?;
                true
            }
            None => {
                //no need to write in `raw_sk`,
                // since the function below will override everything
                // also all 0s is a valid initialization
                false
            }
        };

        cfg_if! {
            if #[cfg(nanox)] {
                let might_throw = || unsafe {
                    crate::raw::cx_ecfp_generate_pair(
                        curve as _,
                        pk.as_mut_ptr(),
                        raw_sk.as_mut_ptr(),
                        keep as u8 as _,
                    );
                };

                catch(might_throw)?;
            } else if #[cfg(nanos)] {
                match unsafe { crate::raw::cx_ecfp_generate_pair_no_throw(
                    curve as _,
                    pk.as_mut_ptr(),
                    raw_sk.as_mut_ptr(),
                    keep,
                )} {
                    0 => (),
                    err => return Err(err.into()),
                }
            } else {
                unimplemented!("generate_ecfp_keypair called in non-bolos");
            }
        }

        //safe because they are both initialized and good pointers
        Ok(unsafe { (Zeroizing::new(raw_sk.assume_init()), pk.assume_init()) })
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
                unimplemented!("cx_ecdsa_sign called in not bolos")
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
                unimplemented!("cx_eddsa_sign called in not bolos")
            }
        }

        Ok(sig_len as usize)
    }
}
use bindings::*;
