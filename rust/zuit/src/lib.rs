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
//! Z UI Test suit
//!
//! This crate provides utilities to run UI elements in our apps
//!
//! Useful for unit or integration testing without having to emulate the devices

use zemu_sys::Viewable;

#[derive(Clone, PartialEq, Eq)]
pub struct Page<const T: usize, const M: usize> {
    pub title: [u8; T],
    pub message: [u8; M],
}

impl<const T: usize, const M: usize> Default for Page<T, M> {
    fn default() -> Self {
        Self {
            title: [0; T],
            message: [0; M],
        }
    }
}

/// This struct will render each item and each page of an item of a given `viewable`
pub struct MockDriver<V, const T: usize, const M: usize> {
    viewable: Box<V>,
    print: bool,

    //[item][page] .title .message
    out: Vec<Vec<Page<T, M>>>,
}

impl<V, const T: usize, const M: usize> MockDriver<V, T, M> {
    pub fn new(viewable: V) -> Self {
        Self {
            viewable: Box::new(viewable),
            print: true,
            out: Default::default(),
        }
    }

    pub fn out_ui(&self) -> &[Vec<Page<T, M>>] {
        self.out.as_slice()
    }

    pub fn with_print(&mut self, print: bool) {
        self.print = print
    }
}

impl<V: Viewable, const T: usize, const M: usize> MockDriver<V, T, M> {
    /// This function allows `callback` to be invoked for each page of each item
    /// that the inner `Viewable` has to offer
    ///
    /// It will also `drive` if there's no data to pass to the callback
    ///
    /// The callback is passed 4 arguments: the item id, the page number, the title and the message
    pub fn verify_with<F, E>(&mut self, mut callback: F) -> Result<(), Vec<E>>
    where
        F: FnMut(usize, usize, &[u8; T], &[u8; M]) -> Result<(), E>,
    {
        if self.out.is_empty() {
            self.drive();
        }

        let mut errors = vec![];
        for (i, item) in self.out.iter().enumerate() {
            for (j, page) in item.iter().enumerate() {
                if let Err(e) = callback(i, j, &page.title, &page.message) {
                    errors.push(e)
                }
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// This function will go thru each page of each item and collect
    /// all the outputs of the viewable
    pub fn drive(&mut self) {
        let num_items = self
            .viewable
            .num_items()
            .expect("unable to retrieve num_items");

        //render each item
        for item_n in 0..num_items {
            //create new containers for this item's pages
            self.out.push(Vec::new());
            let pages = self.out.last_mut().unwrap();

            let mut page_n = 0;
            //set an initial max_page to 255 to make sure we get at least 1 page
            let mut max_pages = 255;

            //render each page of an item
            while page_n < max_pages {
                let mut page = Page::default();

                max_pages = self
                    .viewable
                    .render_item(item_n, &mut page.title[..], &mut page.message[..], page_n)
                    .unwrap_or_else(|e| {
                        panic!(
                            "Error when rendering item #{}, page #{}/#{}; err: {:?}",
                            item_n, page_n, max_pages, e
                        )
                    });

                if self.print {
                    let title = std::str::from_utf8(&page.title[..]).unwrap_or_else(|e| {
                        panic!(
                            "title was not UTF-8; item #{}, page #{}/#{}; err : {:?}",
                            item_n, page_n, max_pages, e
                        )
                    });
                    let message = std::str::from_utf8(&page.message[..]).unwrap_or_else(|e| {
                        panic!(
                            "message was not UTF-8; item #{}, page #{}/#{}; err : {:?}",
                            item_n, page_n, max_pages, e
                        )
                    });

                    println!("{} | {} : {}", item_n, title, message)
                }

                //store page
                pages.push(page);

                //increase counter for next page
                page_n += 1;
            }
        }
    }

    pub fn accept(&mut self, out: &mut [u8]) -> (usize, u16) {
        self.viewable.accept(out)
    }

    pub fn reject(&mut self, out: &mut [u8]) -> (usize, u16) {
        self.viewable.reject(out)
    }
}
