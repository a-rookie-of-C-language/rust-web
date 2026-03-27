use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{parse_macro_input, ItemFn, LitStr};

// ── #[Aspect] ────────────────────────────────────────────────────────────────
/// Marks a struct as an aspect container.  This macro is a no-op pass-through;
/// the real work is done by `#[Before]` / `#[After]` / `#[Around]` on
/// standalone (free) advice functions.
pub fn aspect_impl(_attribute: TokenStream, item: TokenStream) -> TokenStream {
    item
}

// ── #[Before("beanName::methodName")] ────────────────────────────────────────
pub fn before_impl(attribute: TokenStream, item: TokenStream) -> TokenStream {
    advice_impl(attribute, item, "Before")
}

// ── #[After("beanName::methodName")] ─────────────────────────────────────────
pub fn after_impl(attribute: TokenStream, item: TokenStream) -> TokenStream {
    advice_impl(attribute, item, "After")
}

// ── #[Around("beanName::methodName")] ────────────────────────────────────────
pub fn around_impl(attribute: TokenStream, item: TokenStream) -> TokenStream {
    advice_impl(attribute, item, "Around")
}

// ── shared implementation ─────────────────────────────────────────────────────
fn advice_impl(attribute: TokenStream, item: TokenStream, kind_str: &str) -> TokenStream {
    // Parse the pointcut expression argument, e.g. #[Before("userService::save")]
    let pointcut_lit = match syn::parse::<LitStr>(attribute) {
        Ok(lit) => lit,
        Err(e) => return e.to_compile_error().into(),
    };
    let pointcut_str = pointcut_lit.value();

    // Validate "beanName::methodName" format at macro expansion time
    if !pointcut_str.contains("::") {
        return syn::Error::new(
            Span::call_site(),
            "pointcut expression must be in the form \"beanName::methodName\"",
        )
        .to_compile_error()
        .into();
    }

    let func = parse_macro_input!(item as ItemFn);
    let func_ident = &func.sig.ident;

    // Build the AdviceKind token — use spring_boot re-exports so the call-site
    // crate only needs `spring-boot` as a dependency (not `spring-aop` directly).
    let kind_token = match kind_str {
        "Before" => quote! { spring_boot::AdviceKind::Before },
        "After" => quote! { spring_boot::AdviceKind::After  },
        _ => quote! { spring_boot::AdviceKind::Around },
    };

    let pc_lit = LitStr::new(&pointcut_str, Span::call_site());

    // Generate a unique const name to avoid collisions when multiple advice
    // functions are defined in the same scope.
    let submit_ident = proc_macro2::Ident::new(
        &format!("__aspect_registration_{}", func_ident),
        Span::call_site(),
    );

    let expanded = quote! {
        #func

        #[allow(non_upper_case_globals)]
        const #submit_ident: () = {
            inventory::submit! {
                spring_boot::AspectRegistration {
                    pointcut: #pc_lit,
                    kind: #kind_token,
                    handler: |jp: &spring_boot::JoinPoint| #func_ident(jp),
                }
            }
        };
    };

    expanded.into()
}
