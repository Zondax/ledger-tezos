fn main() {
    println!("cargo:rerun-if-env-changed=BOLOS_SDK");

    if let Some(v) = std::env::var_os("BOLOS_SDK") {
        if !v.is_empty() {
            println!("cargo:rustc-cfg=bolos_sdk");
            println!("cargo:warning=[MOCK] BOLOS_SDK is set, not exporting anything")
        }
    }
}
