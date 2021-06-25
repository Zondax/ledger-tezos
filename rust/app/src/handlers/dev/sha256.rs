use std::{convert::TryFrom, prelude::v1::*};

use sha2::digest::Digest;

use crate::{
    constants::{ApduError as Error, APDU_INDEX_INS},
    dispatcher::{ApduHandler, INS_DEV_HASH},
    handlers::{resources::BUFFER, PacketTypes, PacketType},
};

pub struct Sha256;

impl ApduHandler for Sha256 {
    #[inline(never)]
    fn handle(_: &mut u32, tx: &mut u32, _: u32, apdu_buffer: &mut [u8]) -> Result<(), Error> {
        if apdu_buffer[APDU_INDEX_INS] != INS_DEV_HASH {
            return Err(Error::InsNotSupported);
        }
        *tx = 0;

        let packet = PacketTypes::new(apdu_buffer[2], false).map_err(|_| Error::InvalidP1P2)?;
        let len = apdu_buffer[4] as usize;
        if packet.is_init() {
            unsafe {
                BUFFER.lock(Self)?.reset();
                BUFFER
                    .acquire(Self)?
                    .write(&apdu_buffer[5..5 + len])
                    .map_err(|_| Error::DataInvalid)?;
                *tx = 0;

                Ok(())
            }
        } else if packet.is_next() {
            unsafe {
                BUFFER
                    .acquire(Self)?
                    .write(&apdu_buffer[5..5 + len])
                    .map_err(|_| Error::DataInvalid)?;
                *tx = 0;

                Ok(())
            }
        } else if packet.is_last() {
            unsafe {
                BUFFER
                    .acquire(Self)?
                    .write(&apdu_buffer[5..5 + len])
                    .map_err(|_| Error::DataInvalid)?;

                //only read_exact because we don't care about what's in the rest of the buffer
                let digest = sha2::Sha256::digest(BUFFER.acquire(Self)?.read_exact());
                let digest = digest.as_slice();

                if apdu_buffer.len() < digest.len() {
                    return Err(Error::OutputBufferTooSmall);
                }

                apdu_buffer[..digest.len()].copy_from_slice(digest);
                *tx = digest.len() as u32;

                //reset the buffer for next message
                BUFFER.acquire(Self)?.reset();
                BUFFER.release(Self);
                Ok(())
            }
        } else {
            Err(Error::InvalidP1P2)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        assert_error_code,
        dispatcher::{handle_apdu, CLA},
        handlers::ZPacketType,
    };
    use std::convert::TryInto;

    use serial_test::serial;

    #[test]
    #[serial(dev_hash)]
    fn apdu_dev_hash() {
        const MSG: [u8; 0xFF] = [42; 0xFF];

        let mut flags = 0;
        let mut tx = 0;
        let mut buffer = [0; 260];

        //Init
        buffer[0] = CLA;
        buffer[1] = INS_DEV_HASH;
        buffer[2] = ZPacketType::Init.into();
        buffer[3] = 0;
        buffer[4] = 255;
        buffer[5..].copy_from_slice(&MSG[..255]);

        handle_apdu(&mut flags, &mut tx, 260, &mut buffer);
        assert_error_code!(tx, buffer, Error::Success);

        //Add
        MSG[255..].chunks(255).enumerate().for_each(|(i, c)| {
            let len = c.len();
            flags = 0;
            tx = 0;

            buffer[0] = CLA;
            buffer[1] = INS_DEV_HASH;
            buffer[2] = ZPacketType::Add.into();
            buffer[3] = 0;
            buffer[4] = len as u8;

            let msg_sent = (i + 1) * 255; //send 255 bytes * i chunks + 1 (init)
            buffer[5..len].copy_from_slice(&MSG[msg_sent..msg_sent + len]);

            handle_apdu(&mut flags, &mut tx, 5 + len as u32, &mut buffer);
            assert_error_code!(tx, buffer, Error::Success);
        });

        //Last
        flags = 0;
        tx = 0;
        buffer[0] = CLA;
        buffer[1] = INS_DEV_HASH;
        buffer[2] = ZPacketType::Last.into();
        buffer[3] = 0;
        buffer[4] = 0;

        handle_apdu(&mut flags, &mut tx, 5, &mut buffer);
        assert_error_code!(tx, buffer, Error::Success);

        let expected = sha2::Sha256::digest(&MSG[..]);
        let digest = &buffer[..tx as usize - 2];
        assert_eq!(digest, expected.as_slice());
    }

    #[test]
    #[serial(dev_hash)]
    fn apdu_dev_hash_short() {
        const MSG: &[u8] = b"francesco@zondax.ch";
        let len = MSG.len();

        let mut flags = 0;
        let mut tx = 0;
        let mut buffer = [0; 260];

        //Init
        buffer[0] = CLA;
        buffer[1] = INS_DEV_HASH;
        buffer[2] = ZPacketType::Init.into();
        buffer[3] = 0;
        buffer[4] = len as u8;
        buffer[5..5 + len].copy_from_slice(&MSG[..]);

        handle_apdu(&mut flags, &mut tx, 5 + len as u32, &mut buffer);
        assert_error_code!(tx, buffer, Error::Success);

        buffer[0] = CLA;
        buffer[1] = INS_DEV_HASH;
        buffer[2] = ZPacketType::Last.into();
        buffer[3] = 0;
        buffer[4] = 0;

        handle_apdu(&mut flags, &mut tx, 5, &mut buffer);
        assert_error_code!(tx, buffer, Error::Success);

        let expected = sha2::Sha256::digest(&MSG[..]);
        let digest = &buffer[..tx as usize - 2];
        assert_eq!(digest, expected.as_slice());
    }
}
