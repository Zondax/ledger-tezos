use std::{convert::TryFrom, prelude::v1::*};

use once_cell::unsync::Lazy;
use sha2::digest::Digest;

use crate::{
    bolos::{swapping_buffer::SwappingBuffer, PIC},
    constants::{ApduError as Error, APDU_INDEX_INS},
    dispatcher::{ApduHandler, INS_DEV_HASH},
    new_swapping_buffer,
};

const RAM: usize = 0xFF;
const FLASH: usize = 0xFFFF;

type Buffer = SwappingBuffer<'static, 'static, RAM, FLASH>;

static mut BUFFER: PIC<Lazy<Buffer>> = PIC::new(Lazy::new(|| new_swapping_buffer!(RAM, FLASH)));

pub struct Dev {}

#[repr(u8)]
enum PacketType {
    Init = 0,
    Add = 1,
    Last = 2,
}

impl TryFrom<u8> for PacketType {
    type Error = ();

    fn try_from(from: u8) -> Result<Self, ()> {
        match from {
            0 => Ok(Self::Init),
            1 => Ok(Self::Add),
            2 => Ok(Self::Last),
            _ => Err(()),
        }
    }
}

impl Into<u8> for PacketType {
    fn into(self) -> u8 {
        self as _
    }
}

impl ApduHandler for Dev {
    fn handle(_: &mut u32, tx: &mut u32, _: u32, apdu_buffer: &mut [u8]) -> Result<(), Error> {
        if apdu_buffer[APDU_INDEX_INS] != INS_DEV_HASH {
            return Err(Error::InsNotSupported);
        }
        *tx = 0;

        let packet = PacketType::try_from(apdu_buffer[2]).map_err(|_| Error::InvalidP1P2)?;
        let len = apdu_buffer[4] as usize;

        match packet {
            //Reset buffer and start loading data
            PacketType::Init => unsafe {
                BUFFER.reset();
                BUFFER
                    .write(&apdu_buffer[5..5 + len])
                    .map_err(|_| Error::DataInvalid)?;
                *tx = 0;

                Ok(())
            },
            // keep loading data
            PacketType::Add => unsafe {
                BUFFER
                    .write(&apdu_buffer[5..5 + len])
                    .map_err(|_| Error::DataInvalid)?;
                *tx = 0;

                Ok(())
            },
            // load the last bit and perform sha256
            PacketType::Last => unsafe {
                BUFFER
                    .write(&apdu_buffer[5..5 + len])
                    .map_err(|_| Error::DataInvalid)?;

                //only read_exact because we don't care about what's in the rest of the buffer
                let digest = sha2::Sha256::digest(BUFFER.read_exact());
                let digest = digest.as_slice();

                if apdu_buffer.len() < digest.len() {
                    return Err(Error::OutputBufferTooSmall);
                }

                apdu_buffer[..digest.len()].copy_from_slice(digest);
                *tx = digest.len() as u32;

                //reset the buffer for next message
                BUFFER.reset();
                Ok(())
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        dispatcher::{handle_apdu, CLA},
        utils::assert_error_code,
    };

    #[test]
    fn apdu_dev_hash() {
        const MSG: [u8; 0xFF] = [42; 0xFF];

        let mut flags = 0;
        let mut tx = 0;
        let mut buffer = [0; 260];

        //Init
        buffer[0] = CLA;
        buffer[1] = INS_DEV_HASH;
        buffer[2] = PacketType::Init.into();
        buffer[3] = 0;
        buffer[4] = 255;
        buffer[5..].copy_from_slice(&MSG[..255]);

        handle_apdu(&mut flags, &mut tx, 260, &mut buffer);
        assert_error_code(&tx, &buffer, Error::Success);

        //Add
        MSG[255..].chunks(255).enumerate().for_each(|(i, c)| {
            let len = c.len();
            flags = 0;
            tx = 0;

            buffer[0] = CLA;
            buffer[1] = INS_DEV_HASH;
            buffer[2] = PacketType::Add.into();
            buffer[3] = 0;
            buffer[4] = len as u8;

            let msg_sent = (i + 1) * 255; //send 255 bytes * i chunks + 1 (init)
            buffer[5..len].copy_from_slice(&MSG[msg_sent..msg_sent + len]);

            handle_apdu(&mut flags, &mut tx, 5 + len as u32, &mut buffer);
            assert_error_code(&tx, &buffer, Error::Success);
        });

        //Last
        flags = 0;
        tx = 0;
        buffer[0] = CLA;
        buffer[1] = INS_DEV_HASH;
        buffer[2] = PacketType::Last.into();
        buffer[3] = 0;
        buffer[4] = 0;

        handle_apdu(&mut flags, &mut tx, 5, &mut buffer);
        assert_error_code(&tx, &buffer, Error::Success);

        let expected = sha2::Sha256::digest(&MSG[..]);
        let digest = &buffer[..tx as usize - 2];
        assert_eq!(digest, expected.as_slice());
    }

    #[test]
    fn apdu_dev_hash_short() {
        const MSG: &[u8] = b"francesco@zondax.ch";
        let len = MSG.len();

        let mut flags = 0;
        let mut tx = 0;
        let mut buffer = [0; 260];

        //Init
        buffer[0] = CLA;
        buffer[1] = INS_DEV_HASH;
        buffer[2] = PacketType::Last.into();
        buffer[3] = 0;
        buffer[4] = len as u8;
        buffer[5..5 + len].copy_from_slice(&MSG[..]);

        handle_apdu(&mut flags, &mut tx, 5 + len as u32, &mut buffer);
        assert_error_code(&tx, &buffer, Error::Success);

        let expected = sha2::Sha256::digest(&MSG[..]);
        let digest = &buffer[..tx as usize - 2];
        assert_eq!(digest, expected.as_slice());
    }
}
