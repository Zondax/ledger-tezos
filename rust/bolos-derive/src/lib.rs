use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemStatic};

// #[bolos::nvm]
// static mut __FLASH: [[u8; 0xFFFF]; ..];
//
// static mut __FLASH: PIC<NVM<0xFFFF * ..>> = PIC::new(NVM::zeroed());
mod nvm;
#[proc_macro_attribute]
pub fn nvm(metadata: TokenStream, input: TokenStream) -> TokenStream {
    nvm::nvm(metadata, input)
}

// #[bolos::pic]
// static mut BUFFER: MyBuffer = MyBuffer::new();
//
// static mut BUFFER: PIC<MyBuffer> = PIC::new(MyBuffer::new());

#[proc_macro_attribute]
pub fn pic(_: TokenStream, input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as ItemStatic);

    let ItemStatic {
        attrs,
        ident: name,
        mutability,
        ty,
        vis,
        expr,
        ..
    } = input;
    let ty = *ty;
    let expr = *expr;

    let output = quote! {
        #(#attrs)*
        #vis static #mutability #name: ::bolos_sys::PIC<#ty> = ::bolos_sys::PIC::new(#expr);
    };

    output.into()
}

// #[bolos::static]
// static mut LAZY_STATIC_OBJECT: Object = Object::new()
mod lazy_static;

#[proc_macro_attribute]
pub fn lazy_static(metadata: TokenStream, input: TokenStream) -> TokenStream {
    lazy_static::lazy_static(metadata, input)
}
