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
use std::{mem::MaybeUninit, ptr::addr_of_mut};

use crate::{
    constants::{ApduError as Error, BIP32_MAX_LENGTH},
    crypto::Curve,
    dispatcher::ApduHandler,
    handlers::{
        baking::Baking,
        handle_ui_message,
        hwm::{ChainID, WaterMark, HWM},
        public_key::{Addr, GetAddress},
    },
    sys::crypto::bip32::BIP32Path,
    utils::{ApduBufferRead, ApduPanic},
};
use zemu_sys::{Show, ViewError, Viewable};

use core::convert::TryFrom;

use arrayref::array_ref;

pub struct LegacySetup;

impl LegacySetup {
    pub fn setup(
        curve: Curve,
        path: BIP32Path<BIP32_MAX_LENGTH>,
        main_hwm: u32,
        test_hwm: u32,
        chain_id: u32,
        flags: &mut u32,
    ) -> Result<u32, Error> {
        let mut ui = MaybeUninit::uninit();

        SetupUI::new_into(curve, path, main_hwm, test_hwm, chain_id, &mut ui)?;

        unsafe {
            ui.assume_init() //safe since we initialized it above
                .show(flags)
                .map_err(|_| Error::ExecutionError)
                .map(|_| 0)
        }
    }
}

impl ApduHandler for LegacySetup {
    #[inline(never)]
    fn handle<'apdu>(
        flags: &mut u32,
        tx: &mut u32,
        buffer: ApduBufferRead<'apdu>,
    ) -> Result<(), Error> {
        let curve = Curve::try_from(buffer.p2()).map_err(|_| Error::InvalidP1P2)?;

        let cdata = buffer.payload().map_err(|_| Error::DataInvalid)?;
        if cdata.len() < 13 {
            return Err(Error::WrongLength);
        }

        let chain = u32::from_be_bytes(*array_ref!(cdata, 0, 4));
        let main = u32::from_be_bytes(*array_ref!(cdata, 4, 4));
        let test = u32::from_be_bytes(*array_ref!(cdata, 8, 4));

        let path_len = cdata[12] as usize;
        let path = BIP32Path::<BIP32_MAX_LENGTH>::read(
            cdata
                .get(12..1 + 12 + path_len * 4)
                .ok_or(Error::WrongLength)?,
        )
        .map_err(|_| Error::DataInvalid)?;

        *tx = Self::setup(curve, path, main, test, chain, flags)?;

        Ok(())
    }
}

struct SetupUI {
    curve: Curve,
    path: BIP32Path<BIP32_MAX_LENGTH>,
    //reduntant but makes ui faster
    addr: Addr,
    main_hwm: u32,
    test_hwm: u32,
    chain_id: ChainID,
}

impl SetupUI {
    #[inline(never)]
    pub fn new_into(
        curve: Curve,
        path: BIP32Path<BIP32_MAX_LENGTH>,
        main_hwm: u32,
        test_hwm: u32,
        chain_id: u32,
        out: &mut MaybeUninit<Self>,
    ) -> Result<(), Error> {
        crate::sys::zemu_log_stack("SetupUI::new\x00");

        let out = out.as_mut_ptr();
        {
            //get `addr` *mut,
            // cast to MaybeUninit *mut
            //SAFE: `as_mut` it to &mut MaybeUninit (safe because it's MaybeUninit)
            // unwrap the option as it's guarantee valid pointer
            let addr = unsafe { addr_of_mut!((*out).addr).cast::<MaybeUninit<_>>().as_mut() }
                .apdu_unwrap();

            GetAddress::new_addr_into(curve, &path, addr).map_err(|_| Error::ExecutionError)?;
        }

        //actually safe since we are only writing and all pointers are valid
        unsafe {
            addr_of_mut!((*out).curve).write(curve);
            addr_of_mut!((*out).path).write(path);
            addr_of_mut!((*out).main_hwm).write(main_hwm);
            addr_of_mut!((*out).test_hwm).write(test_hwm);
            addr_of_mut!((*out).chain_id).write(chain_id.into());
        }

        Ok(())
    }
}

impl Viewable for SetupUI {
    fn num_items(&mut self) -> Result<u8, ViewError> {
        Ok(5)
    }

    #[inline(never)]
    fn render_item(
        &mut self,
        item_n: u8,
        title: &mut [u8],
        message: &mut [u8],
        page: u8,
    ) -> Result<u8, ViewError> {
        use bolos::{pic_str, PIC};
        use lexical_core::{write as itoa, Number};

        let mut hwm_buf = [0; u32::FORMATTED_SIZE_DECIMAL];

        match item_n {
            0 => {
                let title_content = pic_str!(b"Type");
                title[..title_content.len()].copy_from_slice(title_content);

                handle_ui_message(&pic_str!(b"Setup Baking")[..], message, page)
            }
            1 => {
                let title_content = pic_str!(b"Address");
                title[..title_content.len()].copy_from_slice(title_content);

                let (len, mex) = self.addr.base58();
                handle_ui_message(&mex[..len], message, page)
            }
            2 => {
                let title_content = pic_str!(b"Chain");
                title[..title_content.len()].copy_from_slice(title_content);

                let mut mex = [0; ChainID::BASE58_LEN];
                let len = self
                    .chain_id
                    .to_alias(&mut mex)
                    .map_err(|_| ViewError::Unknown)?;

                handle_ui_message(&mex[..len], message, page)
            }
            3 => {
                let title_content = pic_str!(b"Main Chain HWM");
                title[..title_content.len()].copy_from_slice(title_content);

                handle_ui_message(itoa(self.main_hwm, &mut hwm_buf), message, page)
            }
            4 => {
                let title_content = pic_str!(b"Test Chain HWM");
                title[..title_content.len()].copy_from_slice(title_content);

                handle_ui_message(itoa(self.test_hwm, &mut hwm_buf), message, page)
            }
            _ => Err(ViewError::NoData),
        }
    }

    fn accept(&mut self, out: &mut [u8]) -> (usize, u16) {
        //get public key
        let mut pk = MaybeUninit::uninit();
        if GetAddress::new_key_into(self.curve, &self.path, &mut pk).is_err() {
            return (0, Error::ExecutionError as _);
        }

        //store path & curve for key in memory
        if Baking::store_baking_key(self.curve, self.path).is_err() {
            return (0, Error::ExecutionError as _);
        }

        //set watermaks and chain id
        let main = WaterMark::reset(self.main_hwm, false);
        if HWM::write(main).is_err() {
            return (0, Error::Busy as _);
        }

        let test = WaterMark::reset(self.test_hwm, false);
        if HWM::write_test(test).is_err() {
            return (0, Error::Busy as _);
        }

        if HWM::set_chain_id(self.chain_id.into()).is_err() {
            return (0, Error::Busy as _);
        }

        //write PK to out
        // safe because it's initialized
        let pk = unsafe { pk.assume_init() };
        let key = pk.as_ref();
        let len = key.len();
        out[0] = len as u8;
        out[1..1 + len].copy_from_slice(key);

        (1 + len as usize, Error::Success as _)
    }

    fn reject(&mut self, _: &mut [u8]) -> (usize, u16) {
        (0, Error::CommandNotAllowed as _)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::{
        assert_error_code,
        dispatcher::{handle_apdu, CLA, INS_LEGACY_SETUP},
        sys::get_out,
        utils::MaybeNullTerminatedToString,
    };

    use std::{format, string::ToString};

    use arrayref::array_mut_ref;
    use serial_test::serial;
    use std::convert::TryInto;
    use zuit::{MockDriver, Page};

    #[test]
    #[ignore]
    #[serial(hwm)]
    fn test_setup() {
        let mut flags = 0;
        let mut tx = 0;
        let mut rx = 0;
        let mut buffer = [0; 260];

        let value = 42u32.to_be_bytes();
        let path = BIP32Path::<10>::new([44, 1729, 0, 0].iter().map(|n| 0x8000_0000 + n)).unwrap();
        let path_v = path.serialize();

        //reset state (problematic with other tests)
        HWM::format().expect("couldn't format");

        buffer[rx..rx + 3].copy_from_slice(&[CLA, INS_LEGACY_SETUP, 0]);
        rx += 3;
        buffer[rx] = Curve::Ed25519.into();
        rx += 1;

        //length of payload (all that is below)
        buffer[rx] = 4 * 3 + path_v.len() as u8;
        rx += 1;

        buffer[rx..rx + 4].copy_from_slice(&value[..]); //CHAIN ID
        rx += 4;

        buffer[rx..rx + 4].copy_from_slice(&value[..]); //MAIN HWM
        rx += 4;

        buffer[rx..rx + 4].copy_from_slice(&value[..]); //TEST HWM
        rx += 4;

        buffer[rx..rx + path_v.len()].copy_from_slice(&path_v); //BIP32
        rx += path_v.len();

        handle_apdu(&mut flags, &mut tx, rx as u32, &mut buffer);
        let (_, out) = get_out().expect("UI mock used");

        assert_error_code!(tx, out, Error::Success);

        let pk_len = out[0] as usize;
        assert_eq!(tx as usize, 1 + pk_len + 2);

        match Baking::read_baking_key() {
            Ok(Some((Curve::Ed25519, p))) if p == path => {}
            other => panic!("failed to verify stored baking key: {:?}", other),
        }

        let hwm = HWM::all_hwm().expect("failed retrieving all hwm");
        assert_eq!(&value[..], &hwm[..4]); //main
        assert_eq!(&value[..], &hwm[4..8]); //test
        assert_eq!(&value[..], &hwm[8..12]); //chain_id
    }

    #[test]
    fn setup_ui() {
        let addr = Addr::from_hash(&[0; 20], Curve::Bip32Ed25519).unwrap();
        let (len, addr_base58) = addr.base58();

        let path = BIP32Path::<10>::new([44, 1729, 0, 0].iter().map(|n| 0x8000_0000 + n)).unwrap();

        let expected_ui = [
            ("Type".to_string(), "Setup Baking".to_string()),
            (
                "Address".to_string(),
                std::string::String::from_utf8(addr_base58[..len].to_vec())
                    .expect("addr base58 was not utf8"),
            ),
            ("Chain".to_string(), "".to_string()),
            ("Main Chain HWM".to_string(), format!("{}", 42)),
            ("Test Chain HWM".to_string(), format!("{}", 1)),
        ];

        for chain_id in [
            ChainID::Mainnet,
            ChainID::Any,
            ChainID::Custom(1234),
            ChainID::Custom(420),
        ] {
            let chain_id_alias = {
                let mut alias = [0; ChainID::BASE58_LEN];
                chain_id
                    .to_alias(array_mut_ref!(alias, 0, ChainID::BASE58_LEN))
                    .unwrap();
                alias
                    .to_string_with_check_null()
                    .expect("Chain ID was not UTF8")
            };

            let mut expected_ui = expected_ui.clone();
            expected_ui[2].1 = chain_id_alias;

            let ui = SetupUI {
                curve: Curve::Bip32Ed25519,
                path,
                addr,
                main_hwm: 42,
                test_hwm: 1,
                chain_id,
            };

            let mut driver = MockDriver::<_, 18, 4096>::new(ui);
            driver.drive();

            let produced_ui = driver.out_ui();

            for (item_n, (expected_title, expected_message)) in expected_ui.iter().enumerate() {
                let Page { title, message } = produced_ui[item_n][0]; //we only have 1 page

                let title = title.to_string_with_check_null().unwrap_or_else(|err| {
                    panic!("title from item #{} was not utf8: {:?}", item_n, err)
                });

                //we just check if if starts with since we ignore the paging at the end
                if !title.starts_with(expected_title.as_str()) {
                    panic!(
                        "title for item #{} did not match with expected! title={}; expected={}",
                        item_n, title, expected_title
                    );
                }

                let message = message.to_string_with_check_null().unwrap_or_else(|err| {
                    panic!("message from item #{} was not utf8: {:?}", item_n, err)
                });

                assert_eq!(
                    &message, expected_message,
                    "message for item #{} did not match with expected!",
                    item_n
                )
            }
        }
    }
}
