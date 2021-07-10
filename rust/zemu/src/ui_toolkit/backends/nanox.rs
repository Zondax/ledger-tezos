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
use crate::ui_toolkit::{strlen, ZUI};

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
}

impl Default for NanoXBackend {
    fn default() -> Self {
        Self {
            key: ArrayString::new_const(),
            message: ArrayString::new_const(),
        }
    }
}

impl UIBackend<KEY_SIZE, MESSAGE_SIZE> for NanoXBackend {
    const INCLUDE_ACTIONS_COUNT: usize = 0;

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

    fn view_error_show(&mut self) {
        todo!(
            r#"
        ux_layout_bnnn_paging_reset();
        if (G_ux.stack_count == 0) {
            ux_stack_push();
        }
        ux_flow_init(0, ux_error_flow, NULL);"#
        );
    }

    fn view_review_show(ui: &mut ZUI<Self, KEY_SIZE, MESSAGE_SIZE>) {
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
