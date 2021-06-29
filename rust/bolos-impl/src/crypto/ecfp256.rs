use zeroize::{Zeroize, Zeroizing};

use super::{bip32::BIP32Path, Curve, Mode};
use crate::{
    errors::{catch, Error},
    hash::HasherId,
    misc::FakeLifetimeMut,
    raw::cx_ecfp_private_key_t,
};

#[derive(Debug, Clone, Copy)]
pub struct PublicKey {
    curve: Curve,
    pub len: usize,
    pub w: [u8; 65],
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

pub struct SecretKey {
    curve: Curve,
    pub len: usize,
    pub d: Zeroizing<[u8; 32]>,
}

impl SecretKey {
    fn as_raw_mut(&mut self) -> FakeLifetimeMut<'_, Self, Zeroizing<cx_ecfp_private_key_t>> {
        let curve: u8 = self.curve.into();

        let sk = Zeroizing::new(cx_ecfp_private_key_t {
            curve: curve as u32,
            d_len: self.len as _,
            d: *self.d,
        });

        FakeLifetimeMut::new(self, sk)
    }

    pub fn sign<H>(&mut self, data: &[u8], out: &mut [u8]) -> Result<usize, Error>
    where
        H: HasherId,
        H::Id: Into<u8>,
    {
        let crv = self.curve;
        if crv.is_weirstrass() {
            let (parity, size) = bindings::cx_ecdsa_sign::<H>(self, data, out)?;
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

pub struct Keypair {
    pub public: PublicKey,
    pub secret: SecretKey,
}

impl Keypair {
    pub fn generate<const B: usize>(
        mode: Mode,
        curve: Curve,
        path: &BIP32Path<B>,
    ) -> Result<Self, Error> {
        // Prepare secret key data with the ledger's key
        let mut sk_data = super::bindings::os_perso_derive_node_with_seed_key(mode, curve, path)?;

        // Use the secret key data to prepare a secret key
        let sk_r = cx_ecfp_init_private_key(curve, Some(&sk_data[..]));
        // let's zeroize the sk_data right away before we return
        sk_data.zeroize();

        // bubble up error or get the secret key
        let sk = sk_r?;

        // Use the secret key to generate a keypair
        let (mut sk, pk) = cx_ecfp_generate_pair(curve, Some(sk))?;

        let rs_sk = SecretKey {
            curve,
            len: sk.d_len as usize,
            d: Zeroizing::new(sk.d),
        };
        //erase old sensitive data right away
        sk.d.zeroize();

        let rs_pk = PublicKey {
            curve,
            len: pk.W_len as usize,
            w: pk.W,
        };

        Ok(Self {
            public: rs_pk,
            secret: rs_sk,
        })
    }

    pub fn public(&self) -> &PublicKey {
        &self.public
    }
}

mod bindings {
    use super::{catch, Curve, Error, HasherId, SecretKey};
    use crate::raw::{cx_ecfp_private_key_t, cx_ecfp_public_key_t};
    use zeroize::Zeroize;

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

    pub fn cx_ecfp_generate_pair(
        curve: Curve,
        sk: Option<cx_ecfp_private_key_t>,
    ) -> Result<(cx_ecfp_private_key_t, cx_ecfp_public_key_t), Error> {
        let curve: u8 = curve.into();

        let (mut sk, keep) = match sk {
            Some(sk) => (sk, true),
            None => (Default::default(), false),
        };
        let mut pk = cx_ecfp_public_key_t::default();

        cfg_if! {
            if #[cfg(nanox)] {
                let might_throw = || unsafe {
                    crate::raw::cx_ecfp_generate_pair(
                        curve as _,
                        &mut pk as *mut _,
                        &mut sk as *mut _,
                        keep as u8 as _,
                    );
                };

                if let Err(e) = catch(might_throw) {
                    sk.d.zeroize();
                    return Err(e.into());
                }
            } else if #[cfg(nanos)] {
                match unsafe { crate::raw::cx_ecfp_generate_pair_no_throw(
                    curve as _,
                    &mut pk as *mut _,
                    &mut sk as *mut _,
                    keep,
                )} {
                    0 => (),
                    err => {
                        sk.d.zeroize();
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
    pub fn cx_ecdsa_sign<H>(
        sk: &mut SecretKey,
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

        let mut raw_sk = sk.as_raw_mut();
        let raw_sk = &mut *raw_sk as *const _;

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

    pub fn cx_eddsa_sign(
        sk: &mut SecretKey,
        data: &[u8],
        sig_out: &mut [u8],
    ) -> Result<usize, Error> {
        let id: u8 = crate::hash::Sha512::id().into();

        let crv = sk.curve;

        let mut raw_sk = sk.as_raw_mut();
        let raw_sk = &mut *raw_sk as *const _;

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
