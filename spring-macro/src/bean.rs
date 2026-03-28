use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{parse::Parser, parse_macro_input, ItemFn, LitBool, LitStr, ReturnType, Type};

pub fn bean_impl(attribute: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    let args = match parse_bean_args(attribute) {
        Ok(a) => a,
        Err(e) => return e.to_compile_error().into(),
    };

    let fn_ident = &input.sig.ident;
    let fn_name = fn_ident.to_string();

    // bean 名：优先显式 name=，否则取函数名
    let bean_name = args.name.unwrap_or_else(|| fn_name.clone());
    let name_lit = LitStr::new(&bean_name, Span::call_site());

    // scope
    let scope_token = match args.scope.as_deref().unwrap_or("singleton") {
        "singleton" => quote! { spring_beans::factory::config::BeanScope::Singleton },
        "prototype" => quote! { spring_beans::factory::config::BeanScope::Prototype },
        other => {
            return syn::Error::new(
                Span::call_site(),
                format!(
                    "@Bean scope must be \"singleton\" or \"prototype\", got \"{}\"",
                    other
                ),
            )
            .to_compile_error()
            .into();
        }
    };

    let lazy = args.lazy.unwrap_or(false);

    // 返回类型（必须是 -> SomeType，不能是 -> ()）
    let ret_ty: Box<Type> = match &input.sig.output {
        ReturnType::Type(_, ty) => ty.clone(),
        ReturnType::Default => {
            return syn::Error::new_spanned(&input.sig, "#[Bean] function must have a return type")
                .to_compile_error()
                .into();
        }
    };

    // 保留原函数（供内部调用）
    let original_fn = &input;

    let expanded = quote! {
        // 保留原函数定义
        #original_fn

        // 向全局注册表提交 BeanRegistration
        inventory::submit! {
            spring_beans::registry::BeanRegistration {
                definition: || {
                    spring_beans::factory::config::RootBeanDefinition::new(
                        #name_lit.to_string(),
                        std::any::TypeId::of::<#ret_ty>(),
                        #scope_token,
                        #lazy,
                        vec![],  // @Bean 方法的依赖通过手动调用容器 API 解析（暂不自动推断）
                        Box::new(|_resolved_deps: &std::collections::HashMap<String, std::sync::Arc<dyn std::any::Any + Send + Sync>>, _env: &std::collections::HashMap<String, String>| {
                            let instance = #fn_ident();
                            std::sync::Arc::new(instance) as std::sync::Arc<dyn std::any::Any + Send + Sync>
                        }),
                        None,
                    )
                },
            }
        }
    };
    expanded.into()
}

#[derive(Default)]
struct BeanArgs {
    name: Option<String>,
    scope: Option<String>,
    lazy: Option<bool>,
}

fn parse_bean_args(attribute: TokenStream) -> syn::Result<BeanArgs> {
    let mut args = BeanArgs::default();
    if attribute.is_empty() {
        return Ok(args);
    }
    let parser = syn::meta::parser(|meta| {
        if meta.path.is_ident("name") {
            let v: LitStr = meta.value()?.parse()?;
            args.name = Some(v.value());
            return Ok(());
        }
        if meta.path.is_ident("scope") {
            let v: LitStr = meta.value()?.parse()?;
            args.scope = Some(v.value());
            return Ok(());
        }
        if meta.path.is_ident("lazy") {
            let v: LitBool = meta.value()?.parse()?;
            args.lazy = Some(v.value());
            return Ok(());
        }
        Err(meta.error("unsupported @Bean attribute key"))
    });
    parser.parse(attribute)?;
    Ok(args)
}
