use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{parse_macro_input, spanned::Spanned, Error, Expr, Ident, ItemStatic, Token, Type};

pub fn lazy_static(_: TokenStream, input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as ItemStatic);

    let ItemStatic {
        attrs,
        vis,
        mutability,
        ident: name,
        ty,
        expr,
        ..
    } = input;

    let output = match produce_custom_ty(&name, *ty, *expr, mutability)
        .map_err(|e| e.into_compile_error())
    {
        Err(e) => e,
        Ok(CustomTyOut {
            mod_name,
            struct_name,
            body,
        }) => {
            quote! {
                #body

                #(#attrs)*
                #vis static #mutability #name: self::#mod_name::#struct_name = self::#mod_name::#struct_name::new();
            }
        }
    };

    //eprintln!("{}", output);
    output.into()
}

struct CustomTyOut {
    mod_name: Ident,
    struct_name: Ident,
    body: TokenStream2,
}

fn produce_custom_ty(
    name: &Ident,
    ty: Type,
    init: Expr,
    is_mut: Option<Token![mut]>,
) -> Result<CustomTyOut, Error> {
    let span = name.span();
    let mod_name = Ident::new(&format!("__IMPL_LAZY_{}", name), span);
    let struct_name = Ident::new(&format!("__LAZY_{}", name), span);

    let mut_impl = if is_mut.is_some() {
        quote! {
            impl #struct_name {
               fn get_mut(&mut self) -> &'static mut #ty {
                   self.init();

                   unsafe { LAZY.as_mut_ptr().as_mut().unwrap() }
               }
            }

            impl core::ops::DerefMut for #struct_name {
                fn deref_mut(&mut self) -> &mut Self::Target {
                    self.get_mut()
                }
            }
        }
    } else {
        return Err(Error::new(
            is_mut.span(),
            format!("non-mut static items are not supported!"),
        ));
    };

    let output = quote! {
        #[allow(non_snake_case)]
        #[doc(hidden)]
        mod #mod_name {
            use super::*;
            use ::core::mem::MaybeUninit;

            static mut INITIALIZED: bool = false;
            static mut LAZY: MaybeUninit<#ty> = MaybeUninit::uninit();

            #[allow(non_camel_case_types)]
            pub struct #struct_name {
                __zst: (),
            }

            impl #struct_name {
                pub const fn new() -> Self {
                    Self {
                        __zst: ()
                    }
                }

                #[inline(always)]
                fn init(&self) {
                    #[inline(always)]
                    fn __initialize() -> #ty { #init }

                    let initialized = unsafe { &mut INITIALIZED };

                    if !*initialized {
                        unsafe { LAZY.as_mut_ptr().write(__initialize()); };
                        *initialized = true;
                    }

                }

                fn get(&self) -> &'static #ty {
                    self.init();

                    unsafe { LAZY.as_ptr().as_ref().unwrap() }
                }
            }

            impl core::ops::Deref for #struct_name {
                type Target = #ty;

                fn deref(&self) -> &Self::Target {
                    self.get()
                }
            }

            #mut_impl
        }
    };

    Ok(CustomTyOut {
        mod_name,
        struct_name,
        body: output,
    })
}
