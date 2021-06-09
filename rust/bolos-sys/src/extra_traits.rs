#[cfg(feature = "extra-traits-zeroize")]
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
