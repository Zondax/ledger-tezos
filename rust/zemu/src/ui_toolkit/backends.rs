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
use arrayvec::ArrayString;
use super::ZUI;

pub trait UIBackend<const KEY_SIZE: usize, const MESSAGE_SIZE: usize>: Sized + Default {
    //How many "action" items are we in charge of displaying also
    const INCLUDE_ACTIONS_COUNT: usize;

    fn key_buf(&mut self) -> &mut ArrayString<{ KEY_SIZE }>;

    fn message_buf(&self) -> ArrayString<{ MESSAGE_SIZE }>;

    fn split_value_field(&mut self, message_buf: ArrayString<{ MESSAGE_SIZE }>);

    fn view_error_show(&mut self);

    fn view_review_show(ui: &mut ZUI<Self, KEY_SIZE, MESSAGE_SIZE>);
}

#[cfg(nanos)]
mod nanos;

#[cfg(nanos)]
pub use nanos::NanoSBackend;

#[cfg(nanox)]
mod nanox;

#[cfg(nanox)]
pub use nanox::NanoXBackend;
