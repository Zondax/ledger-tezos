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
use super::UIBackend;
use crate::{
    ui::{manual_vtable::RefMutDynViewable, Viewable},
    ui_toolkit::{strlen, ZUI},
};
use arrayvec::ArrayString;

const KEY_SIZE: usize = 64;
//with null terminator
const MESSAGE_SIZE: usize = 4096;

const INCLUDE_ACTIONS_COUNT: usize = 0;

#[bolos_derive::lazy_static]
pub static mut RUST_ZUI: ZUI<NanoXBackend, KEY_SIZE, MESSAGE_SIZE> = ZUI::new();

pub struct NanoXBackend {
    key: ArrayString<KEY_SIZE>,
    message: ArrayString<MESSAGE_SIZE>,

    viewable_size: usize,
    expert: bool,
}

impl NanoXBackend {
    pub fn update_expert(&mut self) {
        self.message.clear();
        self.message.clear();

        let msg = if self.expert { "enabled" } else { "disabled" };

        write!(self.message, "{}", msg).expect("unable to write expert");
    }

    pub fn toggle_expert(&mut self) {
        self.expert = !self.expert;
    }
}

impl Default for NanoXBackend {
    fn default() -> Self {
        Self {
            key: ArrayString::new_const(),
            message: ArrayString::new_const(),
            viewable_size: 0,
            expert: false,
        }
    }
}

impl UIBackend<KEY_SIZE, MESSAGE_SIZE> for NanoXBackend {
    const INCLUDE_ACTIONS_COUNT: usize = 0;

    fn expert(&self) -> bool {
        self.expert
    }

    fn key_buf(&mut self) -> &mut ArrayString<KEY_SIZE> {
        &mut self.key
    }

    fn message_buf(&self) -> ArrayString<MESSAGE_SIZE> {
        ArrayString::new_const()
    }

    fn split_value_field(&mut self, message_buf: ArrayString<MESSAGE_SIZE>) {
        self.message = message_buf;
        if self.message.len() == 0 {
            self.message.push(' ');
        }
    }

    fn show_idle(&mut self, item_idx: u8, status: Option<&str>) {
        let status = status.unwrap_or("DO NOT USE"); //FIXME: MENU_MAIN_APP_LINE2

        self.key.clear();
        write!(self.key, "{}", status);

        todo!(
            r#"
            if(G_ux.stack_count == 0) {
                ux_stack_push();
            }
            ux_flow_init(0, ux_idle_flow, NULL);
            "#
        );
    }

    fn show_error(&mut self) {
        todo!(
            r#"
        ux_layout_bnnn_paging_reset();
        if (G_ux.stack_count == 0) {
            ux_stack_push();
        }
        ux_flow_init(0, ux_error_flow, NULL);"#
        );
    }

    fn show_review(ui: &mut ZUI<Self, KEY_SIZE, MESSAGE_SIZE>) {
        //reset ui struct
        ui.paging_init();
        //not sure why this is here but ok
        ui.paging_decrease();

        todo!(
            r#"
            flow_inside_loop = 0;
            if G_ux.stack_count == 0 {
                ux_stack_push();
            }
            ux_flow_init(0, ux_review_flow, NULL);
        "#
        );
    }

    fn update_review(ui: &mut ZUI<Self, KEY_SIZE, MESSAGE_SIZE>) {
        match ui.review_update_data() {
            Ok(_) | Err(ViewError::NoData) => {}
            Err(_) => {
                ui.show_error();
            }
        }
    }

    fn wait_ui(&mut self) {
        //FIXME: UX_WAIT
    }

    fn accept_reject_out(&mut self) -> &mut [u8] {
        use bolos_sys::raw::G_io_apdu_buffer as APDU_BUFFER;

        unsafe { &mut APDU_BUFFER[..APDU_BUFFER.len() - self.viewable_size] }
    }

    fn accept_reject_end(&mut self, len: usize) {
        use bolos_sys::raw::{io_exchange, CHANNEL_APDU, IO_RETURN_AFTER_TX};

        io_exchange((CHANNEL_APDU | IO_RETURN_AFTER_TX) as u8, len as u16);
    }

    fn store_viewable<V: Viewable + Sized + 'static>(
        &mut self,
        viewable: V,
    ) -> Option<RefMutDynViewable> {
        use bolos_sys::raw::G_io_apdu_buffer as APDU_BUFFER;

        let size = core::mem::size_of::<V>();
        unsafe {
            let buf_len = APDU_BUFFER.len();
            if size > buf_len {
                return None;
            }

            let new_loc_slice = &mut APDU_BUFFER[buf_len - size..];
            let new_loc_raw_ptr: *mut u8 = new_loc_slice.as_mut_ptr();
            let new_loc: *mut V = new_loc_raw_ptr.cast();

            //write but we don't want to drop `new_loc` since
            // it's not actually valid T data
            core::ptr::write(new_loc, item);

            //write how many bytes we have occupied
            self.viewable_size = size;

            //we can unwrap as we know this ptr is valid
            Some(new_loc.as_mut().unwrap().into())
        }
    }
}

mod cabi {
    use super::*;

    #[no_mangle]
    pub unsafe extern "C" fn viewdata_key() -> *mut u8 {
        RUST_ZUI.backend.key.as_bytes_mut().as_mut_ptr()
    }

    #[no_mangle]
    pub unsafe extern "C" fn viewdata_message() -> *mut u8 {
        RUST_ZUI.backend.message.as_bytes_mut().as_mut_ptr()
    }
}
