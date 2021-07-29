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
    ui_toolkit::ZUI,
};

use arrayvec::ArrayString;

const KEY_SIZE: usize = 64;
const MESSAGE_SIZE: usize = 1024;

#[bolos_derive::lazy_static]
pub static mut RUST_ZUI: ZUI<ConsoleBackend, KEY_SIZE> = ZUI::new();

pub struct ConsoleBackend {
    key: [u8; KEY_SIZE],
    message: ArrayString<MESSAGE_SIZE>,
    expert: bool,
}

impl UIBackend<KEY_SIZE> for ConsoleBackend {
    type MessageBuf = ArrayString<MESSAGE_SIZE>;

    //How many "action" items are we in charge of displaying also
    const INCLUDE_ACTIONS_COUNT: usize = 0;

    fn static_mut() -> &'static mut Self {
        todo!("static_mut")
    }

    fn key_buf(&mut self) -> &mut [u8; KEY_SIZE] {
        &mut self.key
    }

    fn message_buf(&self) -> Self::MessageBuf {
        ArrayString::new_const()
    }

    fn split_value_field(&mut self, message_buf: Self::MessageBuf) {
        self.message = message_buf;
    }

    //view_idle_show_impl
    fn show_idle(&mut self, _item_idx: usize, _status: Option<&[u8]>) {
        todo!("show_idle")
    }

    //view_error_show_impl
    fn show_error(&mut self) {
        todo!("show_error")
    }

    fn show_message(&mut self, _title: &str, _message: &str) {
        todo!("show_message")
    }

    //view_review_show_impl
    fn show_review(_ui: &mut ZUI<Self, KEY_SIZE>) {
        todo!("show_review")
    }

    //h_review_update
    fn update_review(_ui: &mut ZUI<Self, KEY_SIZE>) {
        todo!("update_review")
    }

    //UX_WAIT macro equivalent
    fn wait_ui(&mut self) {
        todo!("wait_ui")
    }

    fn expert(&self) -> bool {
        self.expert
    }

    fn toggle_expert(&mut self) {
        self.expert = !self.expert;
    }

    fn update_expert(&mut self) {
        todo!("update_expert")
    }

    fn accept_reject_out(&mut self) -> &mut [u8] {
        todo!("out")
    }

    fn accept_reject_end(&mut self, _len: usize) {
        todo!("end")
    }

    fn store_viewable<V: Viewable + Sized + 'static>(
        &mut self,
        _viewable: V,
    ) -> Option<RefMutDynViewable> {
        todo!("store_viewable")
    }
}
