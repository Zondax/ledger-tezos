/// Struct representing a BIP32 derivation path, with up to 10 components
pub struct BIP32Path {
    len: u8,
    components: [u32; 10],
}

pub enum BIP32PathError {
    //tried to derive a path with 0 components
    ZeroLength,
    //tried to derive a path with incomplete components
    NotEnoughData,
    //tried to derive a path from an input buffer bigger than requested
    TooMuchData,
}

impl BIP32Path {
    ///Attempt to read a BIP32 Path from the provided input bytes
    pub fn read(input: &[u8]) -> Result<Self, BIP32PathError> {
        let len = input.len();

        if len == 0 {
            return Err(BIP32PathError::ZeroLength);
        } else if len > 10 * 4 {
            return Err(BIP32PathError::TooMuchData);
        } else if len % 4 != 0 {
            return Err(BIP32PathError::NotEnoughData);
        }

        //actual number of components
        let len = len / 4;

        let components = input[..]
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

        let mut components_array = [0; 10];
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