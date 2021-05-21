use zeroize::{Zeroize, Zeroizing};

use super::{bip32::BIP32Path, Curve};
use crate::{
    exceptions::{catch_exception, SyscallError},
    raw::{cx_ecfp_private_key_t, cx_ecfp_public_key_t},
};

#[derive(Debug, Clone, Copy)]
pub struct PublicKey {
    curve: Curve,
    pub len: usize,
    pub w: [u8; 65],
}

impl PublicKey {
    pub fn compress(&self) -> Result<Self, SyscallError> {
        match self.curve {
            Curve::Ed25519 => {
                //create copy, so we don't overwrite a valid public key already
                let mut copy = *self;
                let curve: u8 = self.curve.into();

                //wrap call with possible exception
                let might_throw = || unsafe {
                    crate::raw::cx_edward_compress_point(
                        curve as _,
                        &mut copy.w[0] as *mut u8 as *mut _,
                        copy.len as u32 as _,
                    );
                    //set let to compressed
                    copy.len = 33;
                };
                catch_exception::<SyscallError, _, _>(might_throw)?;

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
    pub fn generate(curve: Curve, path: &BIP32Path) -> Result<Self, SyscallError> {
        //Sensitive data is stored outside so we can clear it in case of an exception
        let mut sk_data = [0u8; 32];
        let mut sk = cx_ecfp_private_key_t::default();

        let might_throw = || {
            let curve: u8 = curve.into();

            // Prepare secret key data with the ledger's key
            unsafe {
                //This shouldn't throw
                crate::raw::os_perso_derive_node_bip32(
                    curve as _,
                    &path.components as *const u32 as *const _,
                    path.len as u32 as _,
                    &mut sk_data[0] as *mut u8 as *mut _,
                    core::ptr::null_mut(),
                );
            }

            unsafe {
                // Use the secret key data to actually get a private key
                crate::raw::cx_ecfp_init_private_key(
                    curve as _,
                    &sk_data[0] as *const u8 as *const _,
                    sk_data.len() as u32 as _,
                    &mut sk as *mut _,
                ); //legacy code ignored the return so...
            }
            sk_data.zeroize();

            let mut pk = cx_ecfp_public_key_t::default();
            unsafe {
                // Use the secret key to generate a keypair,
                // the last parameter dictates wheter
                // the private key is gonna be kept (true)
                // or it will be discarded and a new one generated instead
                crate::raw::cx_ecfp_generate_pair(
                    curve as _,
                    &mut pk as *mut _,
                    &mut sk as *mut _,
                    true as u8 as _,
                );
            }

            pk
        };

        //use retrieved sk and pk to construct keypair
        let pk = match catch_exception::<SyscallError, _, _>(might_throw) {
            Ok(pk) => pk,
            Err(e) => {
                //an exception was thrown, we should zeroize before returning
                sk_data.zeroize();
                sk.d.zeroize();

                return Err(e);
            }
        };

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

#[cfg(bolos_sdk)]
mod bindings {

}
