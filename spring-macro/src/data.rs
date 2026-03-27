use proc_macro::TokenStream;
use quote::quote;
use syn::parse_macro_input;
use syn::{Fields, ItemStruct};

use crate::accessors::{build_getter_methods, build_setter_methods};
pub fn data_impl(_attribute: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemStruct);
    match expand_data(&input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

fn expand_data(input: &ItemStruct) -> syn::Result<proc_macro2::TokenStream> {
    let name = &input.ident;
    let fields = match &input.fields {
        Fields::Named(fields) => &fields.named,
        _ => {
            return Err(syn::Error::new_spanned(
                input,
                "data macro only supports named fields",
            ))
        }
    };
    let mut methods = Vec::new();
    methods.extend(build_getter_methods(fields)?);
    methods.extend(build_setter_methods(fields)?);
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    Ok(quote! {
        #input

        impl #impl_generics #name #ty_generics #where_clause {
            #(#methods)*
        }
    })
}
