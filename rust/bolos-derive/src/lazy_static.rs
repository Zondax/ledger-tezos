/*******************************************************************************
*   (c) 2021 Zondax GmbH
*
*  Licensed under the Apache License, Version 2.0 (the "License");
*  you may not use this file except in compliance with the License.
*  You may obtain a copy of the License at
*
*      http://www.apache.org/licenses/LICENSE-2.0
*
*  Unless required by applicable law or agreed to in writing, software
*  distributed under the License is distributed on an "AS IS" BASIS,
*  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
*  See the License for the specific language governing permissions and
*  limitations under the License.
********************************************************************************/
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{
    parse_macro_input, spanned::Spanned, AttributeArgs, Error, Expr, Ident, ItemStatic, Meta,
    NestedMeta, Token, Type,
};

pub fn lazy_static(metadata: TokenStream, input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(metadata as AttributeArgs);
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

    let output = match produce_custom_ty(
        &name,
        *ty,
        *expr,
        mutability.is_some(),
        is_cbindgen_mode(&args),
    )
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
    is_mut: bool,
    cbindgen: bool,
) -> Result<CustomTyOut, Error> {
    let span = name.span();
    let mod_name = Ident::new(&format!("__IMPL_LAZY_{}", name), span);
    let struct_name = Ident::new(&format!("__LAZY_{}", name), span);
    let static_name = if cbindgen {
        Ident::new(&format!("{}_LAZY", name), span)
    } else {
        Ident::new("LAZY", span)
    };

    let mut_impl = if is_mut {
        quote! {
            impl #struct_name {
               fn get_mut(&mut self) -> &'static mut #ty {
                   self.init();

                   //SAFETY:
                   // same considerations as `get`:
                   // aligned, non-null, initialized by above call
                   // guaranteed single-threaded access
                   unsafe { #static_name.as_mut_ptr().as_mut().unwrap() }
               }
            }

            impl ::core::ops::DerefMut for #struct_name {
                fn deref_mut(&mut self) -> &mut Self::Target {
                    self.get_mut()
                }
            }
        }
    } else {
        return Err(Error::new(
            is_mut.span(),
            "non-mut static items are not supported!".to_string(),
        ));
    };

    let (cbindgen_attrs, cbindgen_vis): (_, Option<Token![pub]>) = if cbindgen {
        (
            quote!(
                #[no_mangle]
            ),
            Some(Default::default()),
        )
    } else {
        (TokenStream2::new(), None)
    };

    let output = quote! {
        #[allow(non_snake_case)]
        #[doc(hidden)]
        mod #mod_name {
            use super::*;
            use ::core::mem::MaybeUninit;

            static mut UNINITIALIZED: MaybeUninit<u8> = MaybeUninit::uninit();

            #cbindgen_attrs
            #cbindgen_vis static mut #static_name: MaybeUninit<#ty> = MaybeUninit::uninit();

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

                fn init(&self) {
                    fn __initialize() -> #ty { #init }

                    //SAFETY:
                    // single-threaded code guarantees no data races when accessing
                    // global variables.
                    // Furthermore, u8 can't be uninitialized as any value is valid.
                    let initialized_ptr = unsafe { UNINITIALIZED.as_mut_ptr() };

                    //SAFETY:
                    // ptr comes from rust so guaranteed to be aligned and not null,
                    // is also initialized (see above), not deallocated (global)
                    let initialized_val = unsafe { ::core::ptr::read_volatile(initialized_ptr as *const _) };

                    if initialized_val != 1u8 {
                        //SAFETY:
                        // single threaded access, non-null, aligned
                        unsafe { #static_name.as_mut_ptr().write(__initialize()); };

                        //SAFETY: see above when reading `initialized_val`
                        unsafe { initialized_ptr.write(1u8); }
                    }

                }

                fn get(&self) -> &'static #ty {
                    self.init();

                    //SAFETY:
                    // code is single-threaed so no data races,
                    // furthermore the pointer is guaranteed to be non-null, aligned
                    // and initialized by the `init` call above
                    unsafe { #static_name.as_ptr().as_ref().unwrap() }
                }
            }

            impl ::core::ops::Deref for #struct_name {
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

// This function looks for the attribute `cbindgen` in the list of attribute
// args given.
// For example, it will return true for
//#[attr(cbindgen)]
#[allow(clippy::ptr_arg)]
fn is_cbindgen_mode(args: &AttributeArgs) -> bool {
    for arg in args {
        if let NestedMeta::Meta(Meta::Path(path)) = arg {
            if path
                .segments
                .iter()
                .any(|path_segment| path_segment.ident == "cbindgen")
            {
                return true;
            }
        }
    }

    false
}
