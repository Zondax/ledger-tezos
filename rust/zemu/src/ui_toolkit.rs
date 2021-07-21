use arrayvec::ArrayString;

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
use backends::UIBackend;
pub use backends::RUST_ZUI;

#[repr(C)]
pub struct ZUI<B: UIBackend<KS, MS> + 'static, const KS: usize, const MS: usize> {
    item_idx: usize,
    item_count: usize,

    page_idx: usize,
    page_count: usize,

    backend: &'static mut B,

    current_viewable: Option<RefMutDynViewable>,
}

impl<B: UIBackend<KS, MS>, const KS: usize, const MS: usize> ZUI<B, KS, MS> {
    pub fn new() -> Self {
        Self {
            item_idx: 0,
            item_count: 0,
            page_idx: 0,
            page_count: 0,
            backend: B::static_mut(),
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

    pub fn paging_can_increase(&self) -> bool {
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

    pub fn paging_can_decrease(&self) -> bool {
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
            self.item_idx -= 1;

            //"jump" to last page, then update will fix this value
            self.page_idx = 255;
        }
    }

    pub fn left_button(&mut self) {
        self.paging_decrease();
        B::update_review(self)
    }

    pub fn right_button(&mut self) {
        self.paging_increase();
        B::update_review(self)
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
    pub fn review_action(&mut self) {
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

        let key_bytes = self.backend.key_buf();

        let render_item_result = viewable.render_item(
            self.item_idx as u8,
            &mut key_bytes[..],
            message_bytes,
            page_idx,
        );

        //asciify
        // this section makes the unsafe above safe!
        message_bytes
            .iter_mut()
            .take_while(|&&mut c| c != 0)
            .filter(|&&mut c| !(32..=0x7F).contains(&c))
            .for_each(|c| {
                *c = b'.';
            });
        key_bytes
            .iter_mut()
            .take_while(|&&mut c| c != 0)
            .filter(|&&mut c| !(32..=0x7F).contains(&c))
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
        use bolos_sys::pic::PIC;

        self.item_count = self
            .current_viewable
            .as_mut()
            .ok_or(ViewError::NoData)?
            .num_items()? as usize + 1;
        self.page_count = 1;

        if self.is_accept_item() {
            //put approve label as message
            // and clear key

            const APPROVE: &str = "APPROVE\x00";
            self.backend.key_buf()[0] = 0;

            let mut tmp = self.backend.message_buf();
            tmp.clear();

            tmp.push_str(PIC::new(APPROVE).into_inner());
            self.backend.split_value_field(tmp);

            self.page_idx = 0;
            return Ok(());
        } else if self.is_reject_item() {
            //put reject label as message
            // and clear key

            const REJECT: &str = "REJECT\x00";
            self.backend.key_buf()[0] = 0;

            let mut tmp = self.backend.message_buf();
            tmp.clear();

            tmp.push_str(PIC::new(REJECT).into_inner());
            self.backend.split_value_field(tmp);

            self.page_idx = 0;
            return Ok(());
        }

        loop {
            //be sure we are not out of bounds
            self.render_item(0)?;

            if self.page_count != 0 && self.page_idx > self.page_count {
                //try again and get last page
                self.page_idx = self.page_count - 1;
            }

            self.render_item(None)?;
            //if we have more than one page, if possible we should display
            // what page we are displaying currently and what's the total number of pages
            self.format_key_with_page();

            if self.page_count != 0 {
                break;
            } else {
                self.paging_increase();
            }
        }

        Ok(())
    }

    fn format_key_with_page(&mut self) {
        if self.page_count > 1 {
            let key = self.backend.key_buf();
            let key_len = strlen(&key[..]);

            if key_len < KS {
                let mut tmp = ArrayString::from_byte_string(&key).expect("key was not utf8");
                tmp.truncate(key_len); //ignore the remaining null bytes (or garbage)

                //this is unrolled equivalent of
                // write!(&mut tmp, " [{}/{}]")
                //if there's any error we return without having changed anything
                use bolos_sys::pic::PIC;

                if tmp.try_push_str(PIC::new(" [").into_inner()).is_err() {
                    return;
                }

                if itoa::fmt(&mut tmp, self.page_idx + 1).is_err() {
                    return;
                }

                if tmp.try_push_str(PIC::new("/").into_inner()).is_err() {
                    return;
                }

                if itoa::fmt(&mut tmp, self.page_count).is_err() {
                    return;
                }

                if tmp.try_push_str(PIC::new("]").into_inner()).is_err() {
                    return;
                }

                //here we have `tmp` with the paging
                //if it fits, then we override key with tmp
                if tmp.len() < KS {
                    key[..tmp.len()].copy_from_slice(tmp.as_bytes());
                }
            }
        }
    }

    //view_error_show
    fn show_error(&mut self) {
        use bolos_sys::pic::PIC;

        const ERROR_KEY: &[u8; 6] = b"ERROR\x00";
        const ERROR_MESSAGE: &str = "SHOWING DATA\x00";

        let key = self.backend.key_buf();
        key[..ERROR_KEY.len()].copy_from_slice(PIC::new(ERROR_KEY).into_inner());

        let mut message = self.backend.message_buf();
        message.clear(); //we want to reset it since it's initialized with zeroes otherwise
        message.push_str(PIC::new(ERROR_MESSAGE).into_inner());

        self.backend.split_value_field(message);

        self.backend.show_error();
    }

    //view_idle_show
    fn show_idle(&mut self, item_idx: usize, status: Option<&[u8]>) {
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

/// This function returns the index of the first null byte in found
pub(self) fn c_strlen(s: *const u8) -> usize {
    let mut count = 0;
    while unsafe { s.add(count).read() } != 0 {
        count += 1;
    }

    count
}
