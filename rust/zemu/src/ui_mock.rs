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

static mut OUT: Option<&mut [u8]> = None;

pub fn set_out(buf: &mut [u8]) {
    unsafe {
        let buf: &'static mut [u8] = core::mem::transmute(buf);
        OUT.replace(buf);
    }
}

impl<T: Viewable + Sized> Show for T {
    unsafe fn show(mut self, _: &mut u32) -> Result<(), ShowTooBig> {
        let out = OUT.as_mut().expect("UI MOCK LAYER NOT INITIALIZED");

        self.accept(out);

        OUT.take();

        Ok(())
    }
}
