use proc_macro::TokenStream;
use quote::quote;
use syn::parse_macro_input;
use syn::{Fields, ItemStruct};

use crate::accessors::build_setter_methods;

pub fn setter_impl(_attribute: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as syn::ItemStruct);
    match expend_setter(input) {
        Ok(expanded) => expanded.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

fn expend_setter(input: ItemStruct) -> syn::Result<proc_macro2::TokenStream> {
    let ident = &input.ident;
    let fields = match &input.fields {
        Fields::Named(fields) => &fields.named,
        _ => {
            return Err(syn::Error::new_spanned(
                input,
                "setter macro only supports named fields",
            ))
        }
    };
    let methods = build_setter_methods(fields)?;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    Ok(quote! {
        #input

        impl #impl_generics #ident #ty_generics #where_clause {
            #(#methods)*
        }
    })
}
