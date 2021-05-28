use proc_macro::TokenStream;
use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input,
    spanned::Spanned,
    Attribute, Error, Ident, ItemStatic, Token, Type, TypeArray, Visibility,
};

// #[bolos::nvm]
// static mut __FLASH: [u8; 0xFFFF];
//
// static mut __FLASH: PIC<NVM<0xFFFF>> = PIC::new(NVM::new());

#[allow(dead_code)]
//nvm attribute input
struct NVMInput {
    attrs: Vec<Attribute>,
    vis: Visibility,
    static_token: Token![static],
    mutability: Option<Token![mut]>,
    name: Ident,
    colon_token: Token![:],
    ty: Box<Type>,
    semi_token: Token![;],
}

impl Parse for NVMInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            attrs: input.call(Attribute::parse_outer)?,
            vis: input.parse()?,
            static_token: input.parse()?,
            mutability: input.parse()?,
            name: input.parse()?,
            colon_token: input.parse()?,
            ty: input.parse()?,
            semi_token: input.parse()?,
        })
    }
}

//helper to add #[link_section] to NVM
struct OnlyOuterAttr(Vec<Attribute>);

impl Parse for OnlyOuterAttr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        input.call(Attribute::parse_outer).map(Self)
    }
}

#[proc_macro_attribute]
pub fn nvm(_: TokenStream, input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as NVMInput);

    let NVMInput {
        mut attrs,
        name,
        mutability,
        ty,
        vis,
        ..
    } = input;
    let ty = *ty;

    //add link_section when in BOLOS
    if cfg!(bolos_sdk) {
        let link_tokens = quote! {#[link_section = ".rodata.N_"]};

        let mut link_attr = syn::parse2::<OnlyOuterAttr>(link_tokens).unwrap().0;
        attrs.append(&mut link_attr);
    }

    let u8_ty = syn::parse2::<Type>(quote! {u8}).unwrap();
    //construct output or error
    let output = match ty {
        Type::Array(TypeArray { len, elem, .. }) if *elem == u8_ty => {
            quote! {
                #(#attrs)*
                #[bolos_sys::pic]
                #vis static #mutability #name: ::bolos_sys::NVM<#len> = ::bolos_sys::NVM::new();
            }
        }
        _ => {
            let ty = quote! {#ty};
            //nvm doesn't handle non-u8 arrays
            Error::new(ty.span(), format!("{} is not an u8 array", ty)).to_compile_error()
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
