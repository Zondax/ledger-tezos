use crate::sys::errors::{catch, throw_raw, Error as SysError};
use crate::{
    constants::{ApduError as Error, APDU_INDEX_INS},
    dispatcher::{ApduHandler, INS_DEV_EXCEPT},
};
use std::convert::TryFrom;

pub struct Except {}

impl ApduHandler for Except {
    #[inline(never)]
    fn handle(_: &mut u32, tx: &mut u32, _: u32, buffer: &mut [u8]) -> Result<(), Error> {
        if buffer[APDU_INDEX_INS] != INS_DEV_EXCEPT {
            return Err(Error::InsNotSupported);
        }
        *tx = 0;

        let do_catch = buffer[2] >= 1;
        let exception = buffer[3];

        #[allow(unreachable_code)]
        let call = move || {
            let ex = match SysError::try_from(exception as u16) {
                Ok(ex) => ex,
                Err(_) => return false,
            };

            throw_raw(ex.into());
            true
        };

        //if we have catch == true, then we should always
        // be returning the passed code
        // otherwise... don't know yet!
        let res = if do_catch { catch(call) } else { Ok(call()) };

        match res {
            //if exception was unspecified, then the call returns false,
            //so we can match against it and return our error
            Ok(false) => {
                return Err(Error::InvalidP1P2);
            }
            Ok(_) => {
                let n: u64 = 0x100000000;
                let n = n.to_be_bytes();
                let len = n.len();
                buffer[..len].copy_from_slice(&n[..]);
                *tx = len as u32;
            }
            Err(ex) => {
                let ex: u32 = ex.into();
                let n = (ex as u64).to_be_bytes();
                let len = n.len();
                buffer[..len].copy_from_slice(&n[..]);
                *tx = len as u32;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        assert_error_code,
        constants::ApduError as Error,
        dispatcher::{handle_apdu, CLA, INS_DEV_EXCEPT},
    };
    use std::convert::TryInto;

    #[test]
    #[should_panic(expected = "exception = 1")]
    fn throw() {
        let mut flags = 0;
        let mut tx = 0;
        let mut buffer = [0; 260];
        let rx = 5;

        let ex: u16 = 1;

        buffer[..rx].copy_from_slice(&[CLA, INS_DEV_EXCEPT, 0, ex as u8, 0]);
        handle_apdu(&mut flags, &mut tx, rx as u32, &mut buffer);
        std::dbg!(&buffer[..tx as usize]);

        assert!(tx > 2);
        assert_error_code!(tx, buffer, Error::Success);
        assert_eq!(&buffer[..2], &1u32.to_be_bytes()[..])
    }

    #[test]
    #[should_panic(expected = "exception = 1")] //unfortunately we don't catch during tests...
    fn catch() {
        let mut flags = 0;
        let mut tx = 0;
        let mut buffer = [0; 260];
        let rx = 5;

        let ex: u16 = 1;

        buffer[..rx].copy_from_slice(&[CLA, INS_DEV_EXCEPT, 1, ex as u8, 0]);
        handle_apdu(&mut flags, &mut tx, rx as u32, &mut buffer);
        std::dbg!(&buffer[..tx as usize]);

        assert!(tx > 2);
        assert_error_code!(tx, buffer, Error::Success);
        //if we don't throw properly we should get this...
        assert_eq!(&buffer[..4], &0x100000000u64.to_be_bytes()[..])
    }
}
