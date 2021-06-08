/// Struct representing a BIP32 derivation path, with up to LEN components
pub struct BIP32Path<const LEN: usize> {
    len: u8,
    components: [u32; LEN],
}

pub enum BIP32PathError {
    //tried to derive a path with 0 components
    ZeroLength,
    //tried to derive a path with incomplete components
    NotEnoughData,
    //tried to derive a path from an input buffer bigger than requested
    TooMuchData,
}

impl<const LEN: usize> BIP32Path<LEN> {
    ///Attempt to read a BIP32 Path from the provided input bytes
    pub fn read(input: &[u8]) -> Result<Self, BIP32PathError> {
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
        } else if len > LEN {
            return Err(BIP32PathError::TooMuchData);
        } else if blen / 4 > len {
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
            .map(|bytes| u32::from_be_bytes(bytes));

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
