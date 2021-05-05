use proc_macro::TokenStream;
use quote::{quote, quote_spanned};
use syn::ItemStatic;

// #[bolos::nvm]
// static mut __FLASH: [u8; 0xFFFF] = [0; 0xFFFF];
//
// static mut __FLASH: PIC<NVM<0xFFFF>> = PIC::new(NVM::new());

struct OnlyOuterAttr(Vec<syn::Attribute>);

impl syn::parse::Parse for OnlyOuterAttr {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        input.call(syn::Attribute::parse_outer).map(Self)
    }
}

#[proc_macro_attribute]
pub fn nvm(_: TokenStream, input: TokenStream) -> TokenStream {
    use syn::{spanned::Spanned, Type, TypeArray};

    let input = syn::parse_macro_input!(input as ItemStatic);

    let ItemStatic {
        mut attrs,
        ident: name,
        mutability,
        ty,
        vis,
        ..
    } = input;

    //add link_section when in BOLOS
    if cfg!(bolos_sdk) {
        let link_tokens = quote! {#[link_section = ".rodata.N_"]}.into();

        let mut link_attr = syn::parse_macro_input!(link_tokens as OnlyOuterAttr).0;
        attrs.append(&mut link_attr);
    }

    let output = if let Type::Array(TypeArray { len, .. }) = *ty {
        quote! {
            #(#attrs)*
            #[bolos_sys::pic]
            #vis static #mutability #name: ::bolos_sys::NVM<#len> = ::bolos_sys::NVM::new();
        }
    } else {
        //nvm doesn't handle arrays
        quote_spanned! {
            ty.span() => compile_error!("not an array")
        }
    };

    output.into()
}

// #[bolos::pic]
// static mut BUFFER: MyBuffer = MyBuffer::new();
//
// static mut BUFFER: PIC<MyBuffer> = PIC::new(MyBuffer::new());

#[proc_macro_attribute]
pub fn pic(_: TokenStream, input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as ItemStatic);

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
