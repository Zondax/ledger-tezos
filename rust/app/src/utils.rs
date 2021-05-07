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
#![allow(dead_code)]

#[macro_export]
macro_rules! assert_error_code {
    ($tx:expr, $buffer:ident, $expected:expr) => {
        let pos: usize = $tx as _;
        let actual: crate::constants::ApduError = (&$buffer[pos - 2..pos]).try_into().unwrap();
        assert_eq!(actual, $expected);
    };
}

git_testament::git_testament_macros!(git);

pub const GIT_COMMIT_HASH: &str = git_commit_hash!();

pub const BAKING: bool = if cfg!(feature = "baking") {
    true
} else {
    false
};
