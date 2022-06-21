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
#![allow(unused_imports, dead_code)]

pub use rslib::{
    constants::{self, ApduError},
    crypto::{self, Curve},
    dispatcher::*,
    rs_handle_apdu, PacketType,
};
use zemu_sys::get_out;

pub use std::convert::TryInto;

use bolos::crypto::bip32::BIP32Path;
use ledger_apdu::APDUAnswer;

pub use ledger_apdu::APDUCommand;

pub fn handle_apdu(flags: &mut u32, tx: &mut u32, rx: u32, apdu_buffer: &mut [u8]) {
    unsafe {
        rs_handle_apdu(
            flags,
            tx,
            rx,
            apdu_buffer.as_mut_ptr(),
            apdu_buffer.len() as u16,
        )
    }
}

pub fn prepare_path<const LEN: usize>(path: &[u32]) -> Vec<u8> {
    assert!(path.len() < 256);

    BIP32Path::<LEN>::new(path.iter().map(|n| 0x8000_0000 + n))
        .unwrap()
        .serialize()
}

pub fn process_apdu(apdu: &APDUCommand<Vec<u8>>) -> APDUAnswer<Vec<u8>> {
    let mut flags = 0u32;
    let mut tx = 0u32;
    let mut buffer = [0u8; 260];

    let serialized_apdu = apdu.serialize();
    buffer[..serialized_apdu.len()].copy_from_slice(&serialized_apdu);

    handle_apdu(
        &mut flags,
        &mut tx,
        serialized_apdu.len() as u32,
        &mut buffer,
    );

    //attempt to retrieve the ui output
    // if none is returned then the show UI was never invoked
    // so all the data is in the apdu buffer
    // otherwise the data is in this buffer
    let buffer_out = match get_out() {
        Some((sz, buf)) => Vec::from(&buf[..sz]),
        None => Vec::from(&buffer[..tx as usize]),
    };

    APDUAnswer::from_answer(buffer_out).unwrap()
}

const USER_MESSAGE_CHUNK_SIZE: usize = 250;

pub fn process_apdu_chunks(
    mut command: APDUCommand<Vec<u8>>,
    message: &[u8],
) -> APDUAnswer<Vec<u8>> {
    let chunks = message.chunks(USER_MESSAGE_CHUNK_SIZE);
    match chunks.len() {
        0 => panic!("empty message"),
        n if n > 255 => panic!("invalid message size"),
        _ => (),
    }

    //make sure p1 is init at first
    command.p1 = PacketType::Init.into();

    let mut answer = process_apdu(&command);
    //no need to check now, we'll check it in the loop

    //set p2 to 0
    command.p2 = 0;

    //start sending chunks
    let last_chunk = chunks.len() - 1;
    for (chunk_idx, chunk) in chunks.enumerate() {
        //return answer if previous errored
        if answer.retcode() != ApduError::Success as u16 {
            return answer;
        }

        println!("sending chunk #{}", chunk_idx);

        //detemine value of p1
        command.p1 = if chunk_idx == last_chunk {
            PacketType::Last
        } else {
            PacketType::Add
        }
        .into();

        //set command data to the current chunk value
        command.data = chunk.to_vec();

        //keep CLA and INS, send new command
        answer = process_apdu(&command);
    }

    // at this point we sent the last
    // chunk so we return the final result to the caller
    answer
}
