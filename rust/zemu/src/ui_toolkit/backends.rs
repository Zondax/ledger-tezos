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
use core::ops::DerefMut;

use crate::ui::{manual_vtable::RefMutDynViewable, Viewable};
use arrayvec::ArrayString;

use super::ZUI;

pub trait UIBackend<const KEY_SIZE: usize>: Sized {
    type MessageBuf: DerefMut<Target = str>;

    //How many "action" items are we in charge of displaying also
    const INCLUDE_ACTIONS_COUNT: usize;

    fn static_mut() -> &'static mut Self;

    fn key_buf(&mut self) -> &mut [u8; KEY_SIZE];

    fn message_buf(&self) -> Self::MessageBuf;

    fn split_value_field(&mut self, message_buf: Self::MessageBuf);

    //view_idle_show_impl
    fn show_idle(&mut self, item_idx: usize, status: Option<&[u8]>);

    //view_error_show_impl
    fn show_error(&mut self);

    //view_message_show_impl
    fn show_message(&mut self, title: &str, message: &str);

    //view_review_show_impl
    fn show_review(ui: &mut ZUI<Self, KEY_SIZE>);

    //h_review_update
    fn update_review(ui: &mut ZUI<Self, KEY_SIZE>);

    //UX_WAIT macro equivalent
    fn wait_ui(&mut self);

    fn expert(&self) -> bool;

    fn toggle_expert(&mut self);

    fn update_expert(&mut self);

    fn accept_reject_out(&mut self) -> &mut [u8];

    fn accept_reject_end(&mut self, len: usize);

    fn store_viewable<V: Viewable + Sized + 'static>(
        &mut self,
        viewable: V,
    ) -> Option<RefMutDynViewable>;
}

cfg_if::cfg_if! {
    if #[cfg(any(nanos, feature = "cbindgen_s"))] {
        mod nanos;
        pub use nanos::{NanoSBackend, RUST_ZUI};
    } else if #[cfg(any(nanox, feature = "cbindgen_x"))] {
        mod nanox;
        pub use nanox::{NanoXBackend, RUST_ZUI};
    } else {
        mod console;
        pub use console::{ConsoleBackend, RUST_ZUI};
    }
}

// #[cfg(nanos)]
// mod nanos;
// #[cfg(nanos)]
// pub use nanos::{NanoSBackend, RUST_ZUI};

// #[cfg(nanox)]
// mod nanox;
// #[cfg(nanox)]
// pub use nanox::{NanoXBackend, RUST_ZUI};
