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
/// Struct representing a BIP32 derivation path, with up to LEN components
#[derive(PartialEq, Eq, Clone, Copy)]
#[cfg_attr(any(feature = "derive-debug", test), derive(Debug))]
#[repr(align(4))]
pub struct BIP32Path<const LEN: usize> {
    len: u8,
    components: [u32; LEN],
}

#[derive(Clone, Copy)]
#[cfg_attr(any(feature = "derive-debug", test), derive(Debug))]
pub enum BIP32PathError {
    //tried to derive a path with 0 components
    ZeroLength,
    //tried to derive a path with incomplete components
    NotEnoughData,
    //tried to derive a path from an input buffer bigger than requested
    TooMuchData,
}

impl<const LEN: usize> BIP32Path<LEN> {
    /// Construct a BIP32Path from a list of components
    pub fn new(components: impl IntoIterator<Item = u32>) -> Result<Self, BIP32PathError> {
        let mut len = 0;
        let mut components_array = [0; LEN];

        for (i, c) in components.into_iter().enumerate() {
            if i > LEN {
                return Err(BIP32PathError::TooMuchData);
            }
            components_array[i] = c;
            len = 1 + i;
        }

        if len == 0 {
            return Err(BIP32PathError::ZeroLength);
        }

        Ok(Self {
            len: len as u8,
            components: components_array,
        })
    }

    ///Attempt to read a BIP32 Path from the provided input bytes
    pub fn read(input: &[u8]) -> Result<Self, BIP32PathError> {
        if input.is_empty() {
            return Err(BIP32PathError::ZeroLength);
        }
        let blen = input.len() - 1;

        if blen == 0 {
            return Err(BIP32PathError::ZeroLength);
        } else if blen % 4 != 0 {
            return Err(BIP32PathError::NotEnoughData);
        }

        //first byte is the number of path components
        let len = input[0] as usize;
        if len == 0 {
            return Err(BIP32PathError::ZeroLength);
        } else if len > LEN || blen / 4 > len {
            return Err(BIP32PathError::TooMuchData);
        } else if blen / 4 < len {
            return Err(BIP32PathError::NotEnoughData);
        }

        //each chunk of 4 bytes thereafter is a path component
        let components = input[1..]
            .chunks(4) //each component is 4 bytes
            .take(len) //take at most `len` chunks
            .map(|c| {
                //conver to array of 4 bytes
                let mut array = [0; 4];
                array.copy_from_slice(c);
                array
            })
            //convert to u32
            .map(u32::from_be_bytes);

        let mut components_array = [0; LEN];
        for (i, component) in components.enumerate() {
            components_array[i] = component;
        }

        Ok(Self {
            len: len as u8,
            components: components_array,
        })
    }

    ///Retrieve the list of components
    pub fn components(&self) -> &[u32] {
        &self.components[..self.len as usize]
    }
}

#[cfg(any(test, feature = "std"))]
impl<const LEN: usize> BIP32Path<LEN> {
    /// Serialize a BIP32Path to a vector, ready to be used on [read](Self::read)
    pub fn serialize(&self) -> std::vec::Vec<u8> {
        let mut v = std::vec::Vec::with_capacity(1 + 4 * self.len as usize);
        v.push(self.len);

        for &p in self.components.iter().take(self.len as usize) {
            v.extend_from_slice(&p.to_be_bytes()[..]);
        }

        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serialize() {
        let expected = BIP32Path::<6>::new([1u32, 2, 3, 4, 5, 6].iter().copied()).unwrap();

        let serialized = expected.serialize();
        let read = BIP32Path::<6>::read(&serialized[..]).expect("can't read serialized");

        assert_eq!(read, expected);
    }
}
