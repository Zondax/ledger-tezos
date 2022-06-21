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
#[path = "ui/comm.rs"]
mod comm;
pub use comm::*;

#[path = "ui/manual_vtable.rs"]
pub(crate) mod manual_vtable;

const UI_OUT_SIZE: usize = 260;

static mut OUT: MockUIHandler<UI_OUT_SIZE> = MockUIHandler::new();

struct MockUIHandler<const SIZE: usize> {
    data: [u8; SIZE],
    written_size: usize,
}

impl<const SIZE: usize> MockUIHandler<SIZE> {
    const INIT_DATA: [u8; SIZE] = [0; SIZE];

    pub const fn new() -> Self {
        Self {
            data: Self::INIT_DATA,
            written_size: 0,
        }
    }

    pub fn flush(&mut self) -> Option<(usize, [u8; SIZE])> {
        if self.written_size != 0 {
            let MockUIHandler { data, written_size } = core::mem::replace(self, Self::new());
            Some((written_size, data))
        } else {
            None
        }
    }

    pub fn as_mut(&mut self) -> &mut [u8; SIZE] {
        &mut self.data
    }

    pub fn set_written(&mut self, n: usize) {
        self.written_size = n;
    }
}

pub fn get_out() -> Option<(usize, [u8; UI_OUT_SIZE])> {
    unsafe { OUT.flush() }
}

impl<T: Viewable + Sized> Show for T {
    unsafe fn show(mut self, _: &mut u32) -> Result<(), ShowTooBig> {
        let out = OUT.as_mut();

        let (len, code) = self.accept(out);

        //write the code to the out buffer manually as
        // it won't be written here but in the apdu buffer otherwise
        out[len..][..2].copy_from_slice(&code.to_be_bytes());
        OUT.set_written(len + 2);

        Ok(())
    }
}
