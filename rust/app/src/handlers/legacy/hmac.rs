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

use crate::handlers::baking::HMAC;
use crate::{
    constants::{ApduError as Error, BIP32_MAX_LENGTH},
    crypto::Curve,
    dispatcher::ApduHandler,
    sys::crypto::bip32::BIP32Path,
    utils::ApduBufferRead,
};

use core::convert::TryFrom;

pub struct LegacyHMAC;

impl ApduHandler for LegacyHMAC {
    #[inline(never)]
    fn handle<'apdu>(
        _: &mut u32,
        tx: &mut u32,
        buffer: ApduBufferRead<'apdu>,
    ) -> Result<(), Error> {
        let curve = Curve::try_from(buffer.p2()).map_err(|_| Error::InvalidP1P2)?;

        let cdata = buffer.payload().map_err(|_| Error::DataInvalid)?;
        if cdata.is_empty() {
            return Err(Error::WrongLength);
        }

        let path_len = cdata[0] as usize;
        let bip32_path = BIP32Path::<BIP32_MAX_LENGTH>::read(
            cdata.get(..1 + 4 * path_len).ok_or(Error::WrongLength)?,
        )
        .map_err(|_| Error::DataInvalid)?;

        *tx = HMAC::hmac(curve, bip32_path, 1 + 4 * path_len, buffer)?;

        Ok(())
    }
}
