use tezos_app::parser::operations::Operation;

fn main() {
    loop {
        honggfuzz::fuzz!(|data: &[u8]| {
            let mut operation = if let Ok(op) = Operation::new(data) {
                op
            } else {
                return;
            };

            let encoded_ops = operation.mut_ops();
            while let Ok(Some(_)) = encoded_ops.parse_next() {}
        });
    }
}
