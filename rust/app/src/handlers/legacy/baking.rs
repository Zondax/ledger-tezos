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
use crate::{
    constants::{ApduError as Error, BIP32_MAX_LENGTH},
    crypto::Curve,
    dispatcher::ApduHandler,
    handlers::baking::{AuthorizeBaking, DeAuthorizeBaking, QueryAuthKey},
    utils::ApduBufferRead,
};

use bolos::crypto::bip32::BIP32Path;
use core::convert::TryFrom;

pub struct LegacyAuthorize;
pub struct LegacyDeAuthorize;
pub struct LegacyQueryAuthKey;
pub struct LegacyQueryAuthKeyWithCurve;

impl ApduHandler for LegacyAuthorize {
    #[inline(never)]
    fn handle<'apdu>(
        flags: &mut u32,
        tx: &mut u32,
        buffer: ApduBufferRead<'apdu>,
    ) -> Result<(), Error> {
        let req_confirmation = buffer.p1() >= 1;

        //confirmation mandatory
        if !req_confirmation {
            return Err(Error::ApduCodeConditionsNotSatisfied);
        }

        let curve = Curve::try_from(buffer.p2()).map_err(|_| Error::InvalidP1P2)?;

        let cdata = buffer.payload().map_err(|_| Error::DataInvalid)?;
        let bip32_path =
            BIP32Path::<BIP32_MAX_LENGTH>::read(cdata).map_err(|_| Error::DataInvalid)?;

        *tx = AuthorizeBaking::authorize(curve, bip32_path, flags)? as u32;

        Ok(())
    }
}

impl ApduHandler for LegacyDeAuthorize {
    #[inline(never)]
    fn handle<'apdu>(
        flags: &mut u32,
        tx: &mut u32,
        buffer: ApduBufferRead<'apdu>,
    ) -> Result<(), Error> {
        *tx = 0;

        let req_confirmation = buffer.p1() >= 1;

        //confirmation mandatory
        if !req_confirmation {
            return Err(Error::ApduCodeConditionsNotSatisfied);
        }

        *tx = DeAuthorizeBaking::deauthorize(flags)?;

        Ok(())
    }
}

impl ApduHandler for LegacyQueryAuthKey {
    #[inline(never)]
    fn handle<'apdu>(
        flags: &mut u32,
        tx: &mut u32,
        buffer: ApduBufferRead<'apdu>,
    ) -> Result<(), Error> {
        let req_confirmation = buffer.p1() >= 1;

        *tx = QueryAuthKey::query(req_confirmation, buffer.write(), flags)?;

        Ok(())
    }
}

impl ApduHandler for LegacyQueryAuthKeyWithCurve {
    #[inline(never)]
    fn handle<'apdu>(
        flags: &mut u32,
        tx: &mut u32,
        buffer: ApduBufferRead<'apdu>,
    ) -> Result<(), Error> {
        let req_confirmation = buffer.p1() >= 1;

        *tx = QueryAuthKey::query(req_confirmation, buffer.write(), flags)?;

        Ok(())
    }
}
