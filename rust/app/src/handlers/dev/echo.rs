use std::{convert::TryFrom, prelude::v1::*};

use crate::{
    constants::{ApduError as Error, APDU_INDEX_INS},
    dispatcher::{ApduHandler, INS_DEV_ECHO_UI},
    sys::{Show, ViewError, Viewable, PIC},
};

#[derive(Default)]
pub struct Echo {
    line1: [u8; 17],
    line2: [u8; 17],
}

impl ApduHandler for Echo {
    #[inline(never)]
    fn handle(_: &mut u32, tx: &mut u32, _: u32, apdu_buffer: &mut [u8]) -> Result<(), Error> {
        if apdu_buffer[APDU_INDEX_INS] != INS_DEV_ECHO_UI {
            return Err(Error::InsNotSupported);
        }
        *tx = 0;

        let mut this: Echo = Default::default();
        let len = apdu_buffer[4] as usize;

        if len > 17 + 17 {
            return Err(Error::WrongLength);
        }

        let first = std::cmp::min(len, 17);
        this.line1[..first].copy_from_slice(&apdu_buffer[5..5 + first]);

        let second = std::cmp::min(len - 17, 17);
        this.line2[..second].copy_from_slice(&apdu_buffer[5 + first..5 + first + second]);

        this.show();

        *tx = 0;
        Ok(())
    }
}

impl Viewable for Echo {
    fn num_items(&mut self) -> Result<u8, ViewError> {
        Ok(1)
    }

    fn render_item(
        &mut self,
        _: u8,
        title: &mut [u8],
        message: &mut [u8],
        _: u8,
    ) -> Result<u8, ViewError> {
        title[..5].copy_from_slice(&PIC::new(b"Echo\x00").into_inner()[..]);

        if message.len() < 17 + 17 + 1  {
            return Err(ViewError::Unknown);
        }
        
        message[..17].copy_from_slice(&self.line1[..]);
        message[17..17 + 17].copy_from_slice(&self.line2[..]);
        message[17 + 17] = 0; //null terminate

        Ok(1)
    }

    fn accept(&mut self) {}

    fn reject(&mut self) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        assert_error_code,
        dispatcher::{handle_apdu, CLA},
    };
    use std::convert::TryInto;

    use serial_test::serial;

    #[test]
    #[serial(dev_hash)]
    fn apdu_dev_echo() {
        const MSG: [u8; 35] = ['a' as u8; 35];

        let mut flags = 0;
        let mut tx = 0;
        let mut buffer = [0; 260];

        buffer[0] = CLA;
        buffer[1] = INS_DEV_ECHO_UI;
        buffer[2] = 0;
        buffer[3] = 0;
        buffer[4] = 35;
        buffer[5..5 + 35].copy_from_slice(&MSG[..35]);

        handle_apdu(&mut flags, &mut tx, 5 + 35, &mut buffer);
        assert_error_code!(tx, buffer, Error::Success);

    }
}
