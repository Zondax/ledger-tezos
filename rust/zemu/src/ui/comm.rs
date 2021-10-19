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
#[derive(Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "derive-debug", derive(Debug))]
pub enum ViewError {
    Unknown,
    NoData,
    Reject,
}

/// This trait describes the interface that is needed for the UI toolkit to
/// show on screen something
pub trait Viewable {
    /// Return the number of items to render
    fn num_items(&mut self) -> Result<u8, ViewError>;

    /// Render `item_n` into `title` and `message`
    ///
    /// If an item is too long to render in the output, the number of "pages" is returned,
    /// and each page can be retrieved via the `page` parameter
    fn render_item(
        &mut self,
        item_n: u8,
        title: &mut [u8],
        message: &mut [u8],
        page: u8,
    ) -> Result<u8, ViewError>;

    /// Called when the last item shown has been "accepted"
    ///
    /// `out` is the apdu_buffer
    ///
    /// Return is number of bytes written to out and the return code
    fn accept(&mut self, out: &mut [u8]) -> (usize, u16);

    /// Called when the last item shows has been "rejected"
    /// `out` is the apdu_buffer
    ///
    /// Return is number of bytes written to out and the return code
    fn reject(&mut self, out: &mut [u8]) -> (usize, u16);
}

pub struct ShowTooBig;

pub trait Show: Viewable + Sized {
    /// This is to be called when you wish to show the item
    ///
    /// `flags` is the same `flags` parameter given in `ApduHandler::handle`
    ///
    /// If an error is returned, then `Self` was too big to fit in the global memory
    ///
    /// # Safety
    /// It's important to return immediately from this function and give control
    /// back to the main loop if the return is Ok
    /// This is also why the function is unsafe, to make sure this postcondition is held
    // for now we consume the item so we can guarantee
    // safe usage
    unsafe fn show(self, flags: &mut u32) -> Result<(), ShowTooBig>;
}
