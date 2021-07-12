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
use crate::{
    ui::{manual_vtable::RefMutDynViewable, Viewable},
    ShowTooBig, ViewError,
};

mod backends;
use self::backends::UIBackend;

pub struct ZUI<B: UIBackend<KS, MS>, const KS: usize, const MS: usize> {
    item_idx: usize,
    item_count: usize,

    page_idx: usize,
    page_count: usize,

    backend: B,

    current_viewable: Option<RefMutDynViewable>,
}

impl<B: UIBackend<KS, MS>, const KS: usize, const MS: usize> ZUI<B, KS, MS> {
    pub fn new() -> Self {
        Self {
            item_idx: 0,
            item_count: 0,
            page_idx: 0,
            page_count: 0,
            backend: B::default(),
            current_viewable: None,
        }
    }

    fn approve(&mut self) {
        self.show_idle(0, None);
        self.backend.wait_ui();

        if let Some(viewable) = self.current_viewable.as_mut() {
            let out = self.backend.accept_reject_out();

            let (len, code) = viewable.accept(out);
            out[len..len + 2].copy_from_slice(&code.to_be_bytes()[..]);

            //remove current viewable
            self.current_viewable.take();

            self.backend.accept_reject_end(len + 2);
        }
    }

    fn reject(&mut self) {
        self.show_idle(0, None);
        self.backend.wait_ui();

        if let Some(viewable) = self.current_viewable.as_mut() {
            let out = self.backend.accept_reject_out();

            let (len, code) = viewable.reject(out);
            out[len..len + 2].copy_from_slice(&code.to_be_bytes()[..]);

            //remove current viewable
            self.current_viewable.take();

            self.backend.accept_reject_end(len + 2);
        }
    }

    fn paging_init(&mut self) {
        self.item_idx = 0;
        self.page_idx = 0;
        self.page_count = 0;
    }

    fn paging_can_increase(&self) -> bool {
        //we have at least 1 page left to show
        let at_least_one_page_left = self.page_idx + 1 < self.page_count;
        //we have at least 1 item, and our current item is not an action
        let at_least_one_non_action_item =
            self.item_count > 0 && self.item_idx < (self.item_count - 1 + B::INCLUDE_ACTIONS_COUNT);

        at_least_one_page_left || at_least_one_non_action_item
    }

    fn paging_increase(&mut self) {
        if self.page_idx + 1 < self.page_count {
            //show next page
            self.page_idx += 1;
        } else if self.item_count > 0
            && self.item_idx < (self.item_count - 1 + B::INCLUDE_ACTIONS_COUNT)
        {
            //show next item
            self.item_idx += 1;
            self.page_idx = 0;
        }
    }

    fn paging_can_decrease(&self) -> bool {
        //not the first page or not the first item
        self.page_idx != 0 || self.item_idx > 0
    }

    fn paging_decrease(&mut self) {
        //if we are not at the first page, then move to previous page
        if self.page_idx != 0 {
            self.page_idx -= 1;
        } else if self.item_idx > 0 {
            //otherwise, since we are already at the first page
            // move to the previous item
            self.item_idx += 1;

            //"jump" to last page, then update will fix this value
            self.page_idx = 255;
        }
    }

    fn is_accept_item(&self) -> bool {
        self.item_idx == self.item_count - 1
    }

    fn set_accept_item(&mut self) {
        self.item_idx = self.item_count - 1;
        self.page_idx = 0;
    }

    fn is_reject_item(&self) -> bool {
        self.item_idx == self.item_count
    }

    //h_review_action
    fn review_action(&mut self) {
        if self.is_accept_item() {
            self.approve();
        } else if self.is_reject_item() {
            self.reject();
        }

        if self.backend.expert() {
            self.set_accept_item();

            B::update_review(self)
        }
    }

    //calls viewable's render_item and makes sure the invariants of the backend are held
    fn render_item(&mut self, page_idx: impl Into<Option<usize>>) -> Result<(), ViewError> {
        let viewable = self.current_viewable.as_mut().ok_or(ViewError::NoData)?;

        let page_idx = page_idx.into().unwrap_or(self.page_idx) as u8;

        let mut message = self.backend.message_buf();

        //Safety: this is unsafe because reading non-UTF8 from str
        // is undefined behaviour, but we will be asciifying the write before
        // we end the borrow, thus making sure it's always valid UTF-8
        let message_bytes = unsafe { message.as_bytes_mut() };

        //Safety: same as per above `message_bytes`
        let key_bytes = unsafe { self.backend.key_buf().as_bytes_mut() };

        let render_item_result =
            viewable.render_item(self.item_idx as u8, key_bytes, message_bytes, page_idx);

        //asciify
        // this section makes the unsafes above safe!
        message_bytes
            .iter_mut()
            .filter(|&&mut c| c != 0 && !(32..=0x7F).contains(&c))
            .for_each(|c| {
                *c = b'.';
            });
        key_bytes
            .iter_mut()
            .filter(|&&mut c| c != 0 && !(32..=0x7F).contains(&c))
            .for_each(|c| {
                *c = b'.';
            });

        //update page count (or return error)
        self.page_count = render_item_result? as usize;

        //let backend split
        self.backend.split_value_field(message);

        Ok(())
    }

    fn review_update_data(&mut self) -> Result<(), ViewError> {
        self.page_count = 1;

        loop {
            self.item_count = self
                .current_viewable
                .as_mut()
                .ok_or(ViewError::NoData)?
                .num_items()? as usize;

            //be sure we are not out of bounds
            self.render_item(0)?;

            if self.page_count != 0 && self.page_idx > self.page_count {
                //try again and get last page
                self.page_idx = self.page_count - 1;
            }

            self.render_item(None)?;

            self.item_count += 1;

            //if we have more than one page, if possible we should display
            // what page we are displaying currently and what's the total number of pages
            if self.page_count > 1 {
                let key = self.backend.key_buf();
                let key_len = strlen(key.as_str().as_bytes());

                if key_len < KS {
                    use core::fmt::Write;
                    //construct temporary new arraystring that will replace the current one
                    let mut tmp = *key;
                    tmp.clear();

                    if write!(
                        tmp,
                        "{} [{}/{}]",
                        &key[..key_len],
                        self.page_idx + 1,
                        self.page_count
                    )
                    .is_ok()
                    {
                        //we override only if ok so that we can keep the original
                        // if an error occured while writing
                        *key = tmp;
                    }
                }
            }

            if self.page_count != 0 {
                break;
            } else {
                self.paging_increase();
            }
        }

        Ok(())
    }

    //view_error_show
    fn show_error(&mut self) {
        use core::fmt::Write;

        let key = self.backend.key_buf();
        key.clear();
        write!(key, "ERROR").expect("unable to write to key");

        let mut message = self.backend.message_buf();
        write!(message, "SHOWING DATA").expect("unable to write message");
        self.backend.split_value_field(message);

        self.backend.show_error();
    }

    //view_idle_show
    fn show_idle(&mut self, item_idx: usize, status: Option<&str>) {
        self.backend.show_idle(item_idx, status)
    }

    //view_review_show
    pub fn show(&mut self, viewable: impl Viewable + Sized + 'static) -> Result<(), ShowTooBig> {
        let viewable = self.backend.store_viewable(viewable).ok_or(ShowTooBig)?;
        self.current_viewable.replace(viewable);

        B::show_review(self);

        Ok(())
    }
}

/// This function returns the index of the first null byte in the slice
fn strlen(s: &[u8]) -> usize {
    let mut count = 0;
    while let Some(&c) = s.get(count) {
        if c == 0 {
            break;
        }
        count += 1;
    }

    count
}
