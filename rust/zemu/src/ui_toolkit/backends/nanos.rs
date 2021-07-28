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
use bolos_derive::pic_str;
use bolos_sys::pic::PIC;

use arrayvec::ArrayString;

pub const KEY_SIZE: usize = 17 + 1;
//with null terminator
pub const MESSAGE_LINE_SIZE: usize = 17 + 1;
const MESSAGE_SIZE: usize = 2 * MESSAGE_LINE_SIZE - 1;

const INCLUDE_ACTIONS_AS_ITEMS: usize = 2;
const INCLUDE_ACTIONS_COUNT: usize = INCLUDE_ACTIONS_AS_ITEMS - 1;

#[bolos_derive::lazy_static]
pub static mut RUST_ZUI: ZUI<NanoSBackend, KEY_SIZE> = ZUI::new();

#[bolos_derive::lazy_static(cbindgen)]
static mut BACKEND: NanoSBackend = NanoSBackend::default();

#[repr(C)]
pub struct NanoSBackend {
    key: [u8; KEY_SIZE],
    value: [u8; MESSAGE_LINE_SIZE],
    value2: [u8; MESSAGE_LINE_SIZE],

    viewable_size: usize,
    expert: bool,
}

impl Default for NanoSBackend {
    fn default() -> Self {
        Self {
            key: [0; KEY_SIZE],
            value: [0; MESSAGE_LINE_SIZE],
            value2: [0; MESSAGE_LINE_SIZE],
            viewable_size: 0,
            expert: false,
        }
    }
}

impl UIBackend<KEY_SIZE> for NanoSBackend {
    type MessageBuf = ArrayString<MESSAGE_SIZE>;

    const INCLUDE_ACTIONS_COUNT: usize = INCLUDE_ACTIONS_COUNT;

    fn static_mut() -> &'static mut Self {
        unsafe { &mut BACKEND }
    }

    fn key_buf(&mut self) -> &mut [u8; KEY_SIZE] {
        &mut self.key
    }

    fn message_buf(&self) -> Self::MessageBuf {
        ArrayString::from_byte_string(&[0; MESSAGE_SIZE]).expect("0x00 is not valid utf8?")
    }

    fn split_value_field(&mut self, message_buf: Self::MessageBuf) {
        //compute len and split `message_buf` at the max line size or at the total len
        // if the total len is less than the size of 1 line

        let len = strlen(message_buf.as_bytes());

        let (line1, line2) = if len >= MESSAGE_LINE_SIZE {
            //we need to split the buffer to fit in 2 lines
            // at LINE_SIZE - 1 since we need to allow line1 to have it's null terminator
            message_buf[..len].split_at(MESSAGE_LINE_SIZE - 1)
        } else {
            //no need to split the buffer, so line 2 will be empty
            (&message_buf[..len], pic_str!(""))
        };

        //write the 2 lines, so if the message was small enough to fit
        // on the first line
        // then the second line will stay empty
        self.value[..line1.len()].copy_from_slice(line1.as_bytes());
        self.value[line1.len()] = 0;

        self.value2[..line2.len()].copy_from_slice(&line2.as_bytes());
        self.value2[line2.len()] = 0; //make sure it's 0 terminated (line1 already is)
    }

    fn show_idle(&mut self, item_idx: usize, status: Option<&[u8]>) {
        //FIXME: MENU_MAIN_APP_LINE2
        let status = status.unwrap_or(&pic_str!(b"DO NOT USE")[..]);

        let len = core::cmp::min(self.key.len(), status.len());
        self.key[..len].copy_from_slice(&status[..len]);

        self.update_expert();

        unsafe {
            bindings::crapoline_ux_menu_display(item_idx as u8);
        }
    }

    fn show_error(&mut self) {
        unsafe {
            bindings::crapoline_ux_display_view_error();
        }
    }

    fn show_message(&mut self, title: &str, message: &str) {
        if let Ok(message) = ArrayString::from(message) {
            self.split_value_field(message);

            let title = title.as_bytes();

            let len = core::cmp::min(self.key.len(), title.len());
            self.key[..len].copy_from_slice(&title[..len]);
        }

        unsafe {
            bindings::crapoline_ux_display_view_message();
        }
    }

    fn show_review(ui: &mut ZUI<Self, KEY_SIZE>) {
        //reset ui struct
        ui.paging_init();

        match ui.review_update_data() {
            Ok(_) => unsafe {
                bindings::crapoline_ux_display_view_review();
            },
            Err(_) => ui.show_error(),
        }
    }

    fn update_review(ui: &mut ZUI<Self, KEY_SIZE>) {
        match ui.review_update_data() {
            Ok(_) => unsafe {
                bindings::crapoline_ux_display_view_review();
            },
            Err(_) => {
                ui.show_error();
                ui.backend.wait_ui();
            }
        }
    }

    fn wait_ui(&mut self) {
        unsafe {
            bindings::crapoline_ux_wait();
        }
    }

    fn expert(&self) -> bool {
        self.expert
    }

    fn toggle_expert(&mut self) {
        self.expert = !self.expert;

        self.show_idle(1, None);
    }

    fn update_expert(&mut self) {
        let msg = if self.expert {
            pic_str!(b"enabled")
        } else {
            pic_str!(b"disabled")
        };

        self.value[..msg.len()].copy_from_slice(msg);
    }

    fn accept_reject_out(&mut self) -> &mut [u8] {
        use bolos_sys::raw::G_io_apdu_buffer as APDU_BUFFER;

        unsafe { &mut APDU_BUFFER[..APDU_BUFFER.len() - self.viewable_size] }
    }

    fn accept_reject_end(&mut self, len: usize) {
        use bolos_sys::raw::{io_exchange, CHANNEL_APDU, IO_RETURN_AFTER_TX};

        // Safety: simple C call
        unsafe {
            io_exchange((CHANNEL_APDU | IO_RETURN_AFTER_TX) as u8, len as u16);
        }
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
            core::ptr::write(new_loc, viewable);

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
    pub unsafe extern "C" fn rs_h_expert_toggle() {
        RUST_ZUI.backend.toggle_expert();
    }

    #[no_mangle]
    pub unsafe extern "C" fn rs_h_expert_update() {
        RUST_ZUI.backend.update_expert();
    }

    #[no_mangle]
    pub unsafe extern "C" fn rs_h_paging_can_decrease() -> bool {
        RUST_ZUI.paging_can_decrease()
    }

    #[no_mangle]
    pub unsafe extern "C" fn rs_h_paging_can_increase() -> bool {
        RUST_ZUI.paging_can_increase()
    }

    #[no_mangle]
    pub unsafe extern "C" fn view_idle_show_impl(item_idx: u8, status: *mut i8) {
        let status = if status.is_null() {
            None
        } else {
            let len = crate::ui_toolkit::c_strlen(status as *const u8);

            Some(unsafe { core::slice::from_raw_parts(status as *const u8, len) })
        };

        RUST_ZUI.show_idle(item_idx as usize, status)
    }

    #[no_mangle]
    pub unsafe extern "C" fn rs_h_error_accept(_: cty::c_uint) {
        RUST_ZUI.accept_error();
    }

    #[no_mangle]
    pub unsafe extern "C" fn rs_h_review_button_both() {
        RUST_ZUI.review_action();
    }

    #[no_mangle]
    pub unsafe extern "C" fn rs_h_review_button_left() {
        RUST_ZUI.left_button();
    }

    #[no_mangle]
    pub unsafe extern "C" fn rs_h_review_button_right() {
        RUST_ZUI.right_button();
    }
}

mod bindings {
    use super::*;

    extern "C" {
        pub fn crapoline_ux_wait();
        pub fn crapoline_ux_menu_display(item_idx: u8);

        pub fn crapoline_ux_display_view_error();
        pub fn crapoline_ux_display_view_review();
        pub fn crapoline_ux_display_view_message();
    }
}
