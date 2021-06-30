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
mod extra_traits_zeroize {
    use zeroize::Zeroize;

    //eventually replace with macro that walks all items in the module
    // and adds `#[derive(Zeroize)]` to all items
    impl Zeroize for crate::raw::cx_ecfp_private_key_t {
        fn zeroize(&mut self) {
            self.d.zeroize();
        }
    }
}
