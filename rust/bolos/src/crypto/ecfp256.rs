use zeroize::{Zeroize, Zeroizing};

use super::{bip32::BIP32Path, Curve, Mode};
use crate::errors::{catch, Error};

#[derive(Debug, Clone, Copy)]
pub struct PublicKey {
    curve: Curve,
    pub len: usize,
    pub w: [u8; 65],
}

impl PublicKey {
    pub fn compress(&self) -> Result<Self, Error> {
        match self.curve {
            Curve::Ed25519 => {
                //create copy, so we don't overwrite a valid public key already
                let mut copy = *self;

                let comp_len = cx_edward_compress_point(self.curve, &mut copy.w[..])?;
                copy.len = comp_len;

                Ok(copy)
            }
            _ => Ok(*self),
        }
    }

    pub fn curve(&self) -> Curve {
        self.curve
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

pub struct Keypair {
    pub public: PublicKey,
    pub secret: SecretKey,
}

impl Keypair {
    pub fn generate(mode: Mode, curve: Curve, path: &BIP32Path) -> Result<Self, Error> {
        // Prepare secret key data with the ledger's key
        let mut sk_data = os_perso_derive_node_with_seed_key::<64>(mode, curve, path)?;

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
    use super::{catch, Mode, BIP32Path, Curve, Error};
    use crate::raw::{cx_ecfp_private_key_t, cx_ecfp_public_key_t};
    use zeroize::Zeroize;

    pub fn cx_edward_compress_point(curve: Curve, p: &mut [u8]) -> Result<usize, Error> {
        let curve: u8 = curve.into();

        cfg_if! {
            if #[cfg(nanox)] {
                let might_throw = || unsafe {
                    crate::raw::cx_edward_compress_point(
                        curve as _,
                        &mut p[0] as *mut u8 as *mut _,
                        p.len() as u32 as _,
                    );
                };

                catch(might_throw)?;
                Ok(33)
            } else if #[cfg(nanos)] {
                match unsafe { crate::raw::cx_edwards_compress_point_no_throw(
                    curve as _,
                    &mut p[0] as *mut u8 as *mut _,
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

        let (sk_data, sk_data_len): (*const u8, u32) = match sk_data {
            None => (std::ptr::null(), 0),
            Some(data) => (&data[0] as *const u8, data.len() as u32),
        };

        let mut out = cx_ecfp_private_key_t::default();

        cfg_if! {
            if #[cfg(nanox)] {
                let might_throw = || unsafe {
                    crate::raw::cx_ecfp_init_private_key(
                        curve as _,
                        sk_data as *const _,
                        sk_data_len as _,
                        &mut out as *mut _,
                    );
                };

                catch(might_throw)?;
            } else if #[cfg(nanos)] {
                match unsafe { crate::raw::cx_ecfp_init_private_key_no_throw(
                    curve as _,
                    sk_data as *const _,
                    sk_data_len as _,
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

    pub fn os_perso_derive_node_with_seed_key<const S: usize>(
        mode: Mode,
        curve: Curve,
        path: &BIP32Path,
    ) -> Result<[u8; S], Error> {
        let curve: u8 = curve.into();
        let mode: u8 = mode.into();

        let mut out = [0; S];
        let out_p = &mut out[0] as *mut u8;
        let (components, path_len) = (&path.components as *const u32, path.len as u32);

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
                todo!("os derive called in non-bolos")
            }
        }

        Ok(out)
    }
}
use bindings::*;
