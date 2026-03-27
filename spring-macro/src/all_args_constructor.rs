use proc_macro::TokenStream;
use quote::quote;
use syn::parse_macro_input;
use syn::{Fields, ItemStruct};

pub fn all_args_constructor_impl(_attribute: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemStruct);
    match expand_all_args_constructor(&input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

fn expand_all_args_constructor(input: &ItemStruct) -> syn::Result<proc_macro2::TokenStream> {
    let ident = &input.ident;
    let fields = match &input.fields {
        Fields::Named(fields) => &fields.named,
        _ => {
            return Err(syn::Error::new_spanned(
                input,
                "all_args_constructor only supports named fields",
            ))
        }
    };
    let params = fields
        .iter()
        .map(|f| {
            let field_ident = f
                .ident
                .as_ref()
                .ok_or_else(|| syn::Error::new_spanned(f, "field must have an identifier"))?;
            let field_type = &f.ty;
            Ok(quote! { #field_ident: #field_type })
        })
        .collect::<syn::Result<Vec<_>>>()?;
    let args = fields.iter().map(|f| f.ident.as_ref().unwrap());
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    Ok(quote! {
        #input

        impl #impl_generics #ident #ty_generics #where_clause {
            pub fn new(#(#params),*) -> Self {
                Self { #(#args),* }
            }
        }
    })
}
