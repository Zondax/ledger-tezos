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
    ui::{manual_vtable::RefMutDynViewable, ViewError, Viewable},
    ui_toolkit::Zui,
};
use bolos_derive::pic_str;
use bolos_sys::pic::PIC;

pub const KEY_SIZE: usize = 63 + 1;
//with null terminator
pub const MESSAGE_SIZE: usize = 4095 + 1;

const INCLUDE_ACTIONS_COUNT: usize = 0;

#[bolos_derive::lazy_static]
pub static mut RUST_ZUI: Zui<NanoSPBackend, KEY_SIZE> = Zui::new();

#[bolos_derive::lazy_static(cbindgen)]
static mut BACKEND: NanoSPBackend = NanoSPBackend::default();

#[repr(C)]
pub struct NanoSPBackend {
    key: [u8; KEY_SIZE],
    message: [u8; MESSAGE_SIZE],

    viewable_size: usize,
    expert: bool,

    flow_inside_loop: bool,
}

impl Default for NanoSPBackend {
    fn default() -> Self {
        Self {
            key: [0; KEY_SIZE],
            message: [0; MESSAGE_SIZE],
            viewable_size: 0,
            expert: false,
            flow_inside_loop: false,
        }
    }
}

impl NanoSPBackend {
    pub fn review_loop_start(&mut self, ui: &mut Zui<Self, KEY_SIZE>) {
        if self.flow_inside_loop {
            //coming from right

            if !ui.paging_can_decrease() {
                //exit to the left
                self.flow_inside_loop = false;
                unsafe {
                    bindings::crapoline_ux_flow_prev();
                }

                return;
            }

            ui.paging_decrease();
        } else {
            ui.paging_init();
        }

        Self::update_review(ui);

        unsafe {
            bindings::crapoline_ux_flow_next();
        }
    }

    pub fn review_loop_end(&mut self, ui: &mut Zui<Self, KEY_SIZE>) {
        if self.flow_inside_loop {
            //coming from left
            ui.paging_increase();

            match ui.review_update_data() {
                Ok(_) => unsafe {
                    bindings::crapoline_ux_layout_bnnn_paging_reset();
                },
                Err(ViewError::NoData) => {
                    self.flow_inside_loop = false;
                    unsafe {
                        bindings::crapoline_ux_flow_next();
                    }
                    return;
                }
                Err(_) => ui.show_error(),
            }
        } else {
            ui.paging_decrease();
            Self::update_review(ui);
        }

        unsafe {
            bindings::crapoline_ux_flow_relayout();
        }
    }
}

impl UIBackend<KEY_SIZE> for NanoSPBackend {
    type MessageBuf = &'static mut str;

    const INCLUDE_ACTIONS_COUNT: usize = 0;

    fn static_mut() -> &'static mut Self {
        unsafe { &mut BACKEND }
    }

    fn update_expert(&mut self) {
        let msg = if self.expert {
            &pic_str!(b"enabled")[..]
        } else {
            &pic_str!(b"disabled")[..]
        };

        self.message[..msg.len()].copy_from_slice(msg);
    }

    fn key_buf(&mut self) -> &mut [u8; KEY_SIZE] {
        &mut self.key
    }

    fn message_buf(&self) -> &'static mut str {
        core::str::from_utf8_mut(&mut Self::static_mut().message)
            //this should never happen as we always asciify
            .expect("message wasn't valid utf8")
    }

    fn split_value_field(&mut self, _: &'static mut str) {}

    fn show_idle(&mut self, _item_idx: usize, status: Option<&[u8]>) {
        let status = status.unwrap_or(&pic_str!(b"DO NOT USE")[..]);

        self.key[..status.len()].copy_from_slice(status);

        unsafe {
            bindings::crapoline_ux_show_idle();
        }
    }

    fn show_error(&mut self) {
        unsafe {
            bindings::crapoline_ux_show_error();
        }
    }

    fn show_message(&mut self, _title: &str, _message: &str) {
        panic!("capability not supported on nanosp yet?")
    }

    fn show_review(ui: &mut Zui<Self, KEY_SIZE>) {
        //reset ui struct
        ui.paging_init();
        //not sure why this is here but ok
        ui.paging_decrease();

        unsafe {
            //we access the backend directly here instead
            // of going thru RUST_ZUI since otherwise we don't have access
            // to this functionality
            BACKEND.flow_inside_loop = false;

            bindings::crapoline_ux_show_review();
        }
    }

    fn update_review(ui: &mut Zui<Self, KEY_SIZE>) {
        match ui.review_update_data() {
            Ok(_) | Err(ViewError::NoData) => {}
            Err(_) => {
                ui.show_error();
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

        unsafe {
            bindings::crapoline_ux_flow_init_idle_flow_toggle_expert();
        }
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
    pub unsafe extern "C" fn view_idle_show_impl(item_idx: u8, status: *mut i8) {
        let status = if status.is_null() {
            None
        } else {
            let len = crate::ui_toolkit::c_strlen(status as *const u8, MESSAGE_SIZE)
                .unwrap_or(MESSAGE_SIZE);

            Some(core::slice::from_raw_parts(status as *const u8, len))
        };

        RUST_ZUI.show_idle(item_idx as usize, status)
    }

    #[no_mangle]
    pub unsafe extern "C" fn rs_h_approve(_: cty::c_uint) {
        RUST_ZUI.approve();
    }

    #[no_mangle]
    pub unsafe extern "C" fn rs_h_reject(_: cty::c_uint) {
        RUST_ZUI.reject();
    }

    #[no_mangle]
    pub unsafe extern "C" fn rs_h_error_accept(_: cty::c_uint) {
        RUST_ZUI.accept_error();
    }

    #[no_mangle]
    pub unsafe extern "C" fn rs_h_review_loop_start() {
        BACKEND.review_loop_start(&mut RUST_ZUI)
    }

    #[no_mangle]
    pub unsafe extern "C" fn rs_h_review_loop_inside() {
        BACKEND.flow_inside_loop = true;
    }

    #[no_mangle]
    pub unsafe extern "C" fn rs_h_review_loop_end() {
        BACKEND.review_loop_end(&mut RUST_ZUI)
    }
}

mod bindings {
    extern "C" {
        pub fn crapoline_ux_wait();
        pub fn crapoline_ux_flow_init_idle_flow_toggle_expert();
        pub fn crapoline_ux_show_review();
        pub fn crapoline_ux_show_error();
        pub fn crapoline_ux_show_idle();
        pub fn crapoline_ux_flow_prev();
        pub fn crapoline_ux_flow_next();
        pub fn crapoline_ux_layout_bnnn_paging_reset();
        pub fn crapoline_ux_flow_relayout();
    }
}
