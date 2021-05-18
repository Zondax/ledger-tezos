pub struct BIP32Path {
    len: u8,
    components: [u32; 10],
}

pub enum BIP32PathError {
    //tried to derive a path with 0 components
    ZeroLength,
}

impl BIP32Path {
    ///Attempt to read a BIP32 Path from the provided input bytes
    pub fn read(input: &[u8]) -> Result<Self, BIP32PathError> {
        let len = input[0];

        if len == 0 {
            return Err(BIP32PathError::ZeroLength);
        }

        let components = input[1..]
            .chunks(4) //each component is 4 bytes
            .take(len as usize) //take at most `len` chunks
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
            len,
            components: components_array,
        })
    }
}
