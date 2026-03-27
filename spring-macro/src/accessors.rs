use proc_macro2::Span;
use quote::quote;
use syn::punctuated::Punctuated;
use syn::token::Comma;
use syn::{Field, Ident};

pub fn build_getter_methods(
    fields: &Punctuated<Field, Comma>,
) -> syn::Result<Vec<proc_macro2::TokenStream>> {
    let mut methods = Vec::new();
    for field in fields {
        let field_ident = field
            .ident
            .as_ref()
            .ok_or_else(|| syn::Error::new_spanned(field, "field must have an identifier"))?;
        let field_name = field_ident.to_string();
        let field_type = &field.ty;
        let getter_name = Ident::new(&format!("get_{}", field_name), Span::call_site());
        methods.push(quote! {
            pub fn #getter_name(&self) -> &#field_type {
                &self.#field_ident
            }
        });
    }
    Ok(methods)
}

pub fn build_setter_methods(
    fields: &Punctuated<Field, Comma>,
) -> syn::Result<Vec<proc_macro2::TokenStream>> {
    let mut methods = Vec::new();
    for field in fields {
        let field_ident = field
            .ident
            .as_ref()
            .ok_or_else(|| syn::Error::new_spanned(field, "field must have an identifier"))?;
        let field_name = field_ident.to_string();
        let field_type = &field.ty;
        let setter_name = Ident::new(&format!("set_{}", field_name), Span::call_site());
        methods.push(quote! {
            pub fn #setter_name(&mut self, value: #field_type) {
                self.#field_ident = value;
            }
        });
    }
    Ok(methods)
}
