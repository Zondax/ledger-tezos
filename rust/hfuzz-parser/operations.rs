use rslib::parser::operations::Operation;

fn main() {
    loop {
        honggfuzz::fuzz!(|data: &[u8]| {
            if let Ok(mut op) = Operation::new(data) {
                let encoded_ops = op.mut_ops();
                while let Ok(Some(_)) = encoded_ops.parse_next() {}
            }
        });
    }
}
