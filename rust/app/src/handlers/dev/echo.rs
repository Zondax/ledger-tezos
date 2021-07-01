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
use std::prelude::v1::*;

use crate::{
    constants::ApduError as Error,
    dispatcher::{ApduHandler, INS_DEV_ECHO_UI},
    sys::{Show, ViewError, Viewable, PIC},
    utils::ApduBufferRead,
};

#[derive(Default)]
pub struct Echo {
    line1: [u8; 17],
    line2: [u8; 17],
}

impl ApduHandler for Echo {
    #[inline(never)]
    fn handle<'apdu>(
        flags: &mut u32,
        tx: &mut u32,
        apdu_buffer: ApduBufferRead<'apdu>,
    ) -> Result<(), Error> {
        *tx = 0;
        if apdu_buffer.ins() != INS_DEV_ECHO_UI {
            return Err(Error::InsNotSupported);
        }

        let mut this: Echo = Default::default();
        let payload = apdu_buffer.payload().map_err(|_| Error::DataInvalid)?;
        let len = payload.len();

        if len > 17 + 17 {
            return Err(Error::WrongLength);
        }

        let first = std::cmp::min(len, 17);
        this.line1[..first].copy_from_slice(&payload[..first]);

        let second = std::cmp::min(len - 17, 17);
        this.line2[..second].copy_from_slice(&payload[first..first + second]);

        match unsafe { this.show(flags) } {
            Ok(_) => Ok(()),
            Err(_) => Err(Error::ExecutionError),
        }
    }
}

impl Viewable for Echo {
    fn num_items(&mut self) -> Result<u8, ViewError> {
        Ok(1)
    }

    fn render_item(
        &mut self,
        idx: u8,
        title: &mut [u8],
        message: &mut [u8],
        _: u8,
    ) -> Result<u8, ViewError> {
        if let 0 = idx {
            title[..5].copy_from_slice(&PIC::new(b"Echo\x00").into_inner()[..]);

            if message.len() < 17 + 17 + 1 {
                return Err(ViewError::Unknown);
            }

            message[..17].copy_from_slice(&self.line1[..]);
            message[17..17 + 17].copy_from_slice(&self.line2[..]);
            message[17 + 17] = 0; //null terminate

            Ok(1)
        } else {
            Err(ViewError::NoData)
        }
    }

    fn accept(&mut self, _: &mut [u8]) -> (usize, u16) {
        (0, Error::Success as _)
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
        dispatcher::{handle_apdu, CLA},
        sys::set_out,
    };
    use std::convert::TryInto;

    use serial_test::serial;

    #[test]
    #[serial(ui)]
    fn apdu_dev_echo() {
        const MSG: [u8; 34] = [b'a'; 34];

        let mut flags = 0;
        let mut tx = 0;
        let mut buffer = [0; 260];

        buffer[0] = CLA;
        buffer[1] = INS_DEV_ECHO_UI;
        buffer[2] = 0;
        buffer[3] = 0;
        buffer[4] = 34;
        buffer[5..5 + 34].copy_from_slice(&MSG[..34]);

        set_out(&mut buffer);
        handle_apdu(&mut flags, &mut tx, 5 + 34, &mut buffer);
        assert_error_code!(tx, buffer, Error::Success);
    }
}
