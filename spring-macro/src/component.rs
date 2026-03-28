use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::parse::Parser;
use syn::{
    parse_macro_input, Attribute, Expr, ExprArray, ExprLit, Fields, GenericArgument, Ident,
    ItemStruct, Lit, LitBool, LitStr, PathArguments, Type,
};

pub fn component_impl(attribute: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemStruct);
    let args = match parse_component_args(attribute) {
        Ok(args) => args,
        Err(err) => return err.to_compile_error().into(),
    };
    let ident = &input.ident;
    let default_name = default_bean_name(ident);
    let name = args.name.unwrap_or(default_name);
    // 从 struct attrs 中读取 #[Scope(...)] / #[Lazy]，覆盖显式参数
    let struct_scope = extract_scope_attr(&input.attrs);
    let struct_lazy = extract_lazy_attr(&input.attrs);
    let scope = struct_scope
        .or(args.scope)
        .unwrap_or_else(|| "singleton".to_string());
    let lazy = struct_lazy.or(args.lazy).unwrap_or(false);
    let name_lit = LitStr::new(&name, Span::call_site());
    let scope_token = match scope.as_str() {
        "singleton" => quote! { spring_beans::factory::config::BeanScope::Singleton },
        "prototype" => quote! { spring_beans::factory::config::BeanScope::Prototype },
        _ => {
            return syn::Error::new_spanned(&input, "scope must be \"singleton\" or \"prototype\"")
                .to_compile_error()
                .into()
        }
    };
    // 无条件扫描 #[autowired] 字段，无需 autowire=true 参数（Spring 风格）
    let deps_list = if !args.deps.is_empty() {
        args.deps
    } else {
        collect_autowired_fields(&input)
            .iter()
            .map(|(_, bean_name, _)| bean_name.clone())
            .collect()
    };

    let deps: Vec<LitStr> = deps_list
        .iter()
        .map(|dep| LitStr::new(dep, Span::call_site()))
        .collect();

    // 无条件生成注入语句
    // 无条件生成注入语句（autowired + Value）
    let inject_stmts = build_inject_stmts(&input);
    let value_inject_stmts = build_value_inject_stmts(&input);

    // 读取 #[ConditionalOnProperty("key", having = "value")] 条件
    let condition_token = match extract_conditional_attr(&input.attrs) {
        Some((key, val)) => {
            let k = LitStr::new(&key, Span::call_site());
            let v = LitStr::new(&val, Span::call_site());
            quote! { Some((#k.to_string(), #v.to_string())) }
        }
        None => quote! { None },
    };

    // 剥离 struct 字段上的 #[autowired] 属性，避免编译器找不到该 helper attribute
    let clean_input = strip_helper_attrs(input.clone());
    let expanded = quote! {
        #clean_input
        impl #ident {
            pub fn bean_name() -> &'static str {
                #name_lit
            }

            pub fn bean_definition() -> spring_beans::factory::config::RootBeanDefinition {
                spring_beans::factory::config::RootBeanDefinition::new(
                    #name_lit.to_string(),
                    std::any::TypeId::of::<#ident>(),
                    #scope_token,
                    #lazy,
                    vec![#(#deps.to_string()),*],
                    Box::new(|resolved_deps: &std::collections::HashMap<String, std::sync::Arc<dyn std::any::Any + Send + Sync>>, env: &std::collections::HashMap<String, String>| {
                        let mut instance = #ident::default();
                        #(#inject_stmts)*
                        #(#value_inject_stmts)*
                        std::sync::Arc::new(instance) as std::sync::Arc<dyn std::any::Any + Send + Sync>
                    }),
                    #condition_token,
                )
            }
        }
        // 编译期自动向全局注册表提交一条记录，Application::run() 启动时自动扫描
        inventory::submit! {
            spring_beans::registry::BeanRegistration {
                definition: || #ident::bean_definition(),
            }
        }
    };
    expanded.into()
}

pub fn component_derive_impl(item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemStruct);
    let ident = &input.ident;
    let name = default_bean_name(ident);
    let name_lit = LitStr::new(&name, Span::call_site());

    // 收集所有带 #[autowired] 的字段信息用于生成 deps 列表和注入代码
    let autowired_fields = collect_autowired_fields(&input);
    let deps: Vec<LitStr> = autowired_fields
        .iter()
        .map(|(_, bean_name, _)| LitStr::new(bean_name, Span::call_site()))
        .collect();

    let inject_stmts = build_inject_stmts(&input);
    let value_inject_stmts = build_value_inject_stmts(&input);

    let expanded = quote! {
        impl #ident {
            pub fn bean_name() -> &'static str {
                #name_lit
            }

            pub fn bean_definition() -> spring_beans::factory::config::RootBeanDefinition {
                spring_beans::factory::config::RootBeanDefinition::new(
                    #name_lit.to_string(),
                    std::any::TypeId::of::<#ident>(),
                    spring_beans::factory::config::BeanScope::Singleton,
                    false,
                    vec![#(#deps.to_string()),*],
                    Box::new(|resolved_deps: &std::collections::HashMap<String, std::sync::Arc<dyn std::any::Any + Send + Sync>>, env: &std::collections::HashMap<String, String>| {
                        let mut instance = #ident::default();
                        #(#inject_stmts)*
                        #(#value_inject_stmts)*
                        std::sync::Arc::new(instance) as std::sync::Arc<dyn std::any::Any + Send + Sync>
                    }),
                    None,
                )
            }
        }
    };
    expanded.into()
}

/// 收集带 #[autowired] 的字段：(字段名 Ident, bean name String, 字段类型 Type)
fn collect_autowired_fields(input: &ItemStruct) -> Vec<(Ident, String, Type)> {
    let mut result = Vec::new();
    let fields = match &input.fields {
        Fields::Named(fields) => &fields.named,
        _ => return result,
    };
    for field in fields {
        if field
            .attrs
            .iter()
            .any(|attr| attr.path().is_ident("autowired"))
        {
            let field_ident = match field.ident.clone() {
                Some(id) => id,
                None => continue,
            };
            if let Some(bean_name) = extract_dependency_name(&field.ty) {
                result.push((field_ident, bean_name, field.ty.clone()));
            }
        }
    }
    result
}

/// 为每个 #[autowired] 字段生成 resolved_deps 取值 + downcast + clone 的注入语句
fn build_inject_stmts(input: &ItemStruct) -> Vec<proc_macro2::TokenStream> {
    collect_autowired_fields(input)
        .into_iter()
        .map(|(field_ident, bean_name, field_ty)| {
            let bean_name_lit = LitStr::new(&bean_name, Span::call_site());
            quote! {
                instance.#field_ident = resolved_deps
                    .get(#bean_name_lit)
                    .and_then(|b| b.as_ref().downcast_ref::<#field_ty>())
                    .map(Clone::clone)
                    .unwrap_or_else(|| panic!(
                        "autowire failed: bean '{}' not found or type mismatch for field '{}'",
                        #bean_name_lit,
                        stringify!(#field_ident)
                    ));
            }
        })
        .collect()
}

/// 收集带 #[Value("${key:default}")] 的字段：(字段名 Ident, placeholder String, 字段类型 Type)
fn collect_value_fields(input: &ItemStruct) -> Vec<(Ident, String, Type)> {
    let mut result = Vec::new();
    let fields = match &input.fields {
        Fields::Named(fields) => &fields.named,
        _ => return result,
    };
    for field in fields {
        for attr in &field.attrs {
            if path_has_ident(attr.path(), "Value") || path_has_ident(attr.path(), "value") {
                if let Ok(lit) = attr.parse_args::<LitStr>() {
                    let placeholder = lit.value();
                    if let Some(field_ident) = field.ident.clone() {
                        result.push((field_ident, placeholder, field.ty.clone()));
                    }
                }
            }
        }
    }
    result
}

/// 为每个 #[Value("${key:default}")] 字段生成 env 读取 + parse 注入语句
fn build_value_inject_stmts(input: &ItemStruct) -> Vec<proc_macro2::TokenStream> {
    collect_value_fields(input)
        .into_iter()
        .map(|(field_ident, placeholder, field_ty)| {
            let placeholder_lit = LitStr::new(&placeholder, Span::call_site());

            // ── SpEL path: #{expr} ──────────────────────────────────────
            if let Some(spel_expr) = placeholder
                .strip_prefix("#{")
                .and_then(|s| s.strip_suffix('}'))
            {
                let spel_lit = LitStr::new(spel_expr.trim(), Span::call_site());
                return quote! {
                    instance.#field_ident = {
                        let _raw = spring_boot::spel::eval(#spel_lit, env)
                            .unwrap_or_else(|e| panic!(
                                "#[Value] SpEL evaluation failed for expression '{}': {}",
                                #spel_lit, e
                            ));
                        _raw.parse().unwrap_or_else(|_| panic!(
                            "#[Value] failed to parse SpEL result '{}' (expression '{}') as {}",
                            _raw,
                            #spel_lit,
                            stringify!(#field_ty)
                        ))
                    };
                };
            }

            // ── property placeholder path: ${key:default} ───────────────
            let (key, default_val) = parse_placeholder(&placeholder);
            let key_lit = LitStr::new(&key, Span::call_site());
            let default_lit = LitStr::new(&default_val, Span::call_site());
            quote! {
                instance.#field_ident = {
                    let _raw = env
                        .get(#key_lit)
                        .map(|s| s.as_str())
                        .unwrap_or(#default_lit);
                    _raw.parse().unwrap_or_else(|_| panic!(
                        "#[Value] failed to parse property '{}' (placeholder '{}') as {}",
                        #key_lit,
                        #placeholder_lit,
                        stringify!(#field_ty)
                    ))
                };
            }
        })
        .collect()
}

fn path_has_ident(path: &syn::Path, ident: &str) -> bool {
    path.is_ident(ident)
        || path
            .segments
            .last()
            .map(|segment| segment.ident == ident)
            .unwrap_or(false)
}

/// Parse `${key:default}` or `${key}` into (key, default) where default is "" when absent.
fn parse_placeholder(placeholder: &str) -> (String, String) {
    let inner = placeholder
        .strip_prefix("${")
        .and_then(|s| s.strip_suffix('}'))
        .unwrap_or(placeholder);
    if let Some(pos) = inner.find(':') {
        (inner[..pos].to_string(), inner[pos + 1..].to_string())
    } else {
        (inner.to_string(), String::new())
    }
}

#[derive(Default)]
struct ComponentArgs {
    name: Option<String>,
    scope: Option<String>,
    lazy: Option<bool>,
    deps: Vec<String>,
}

fn parse_component_args(attribute: TokenStream) -> syn::Result<ComponentArgs> {
    let mut args = ComponentArgs::default();
    let parser = syn::meta::parser(|meta| {
        if meta.path.is_ident("name") {
            let value: LitStr = meta.value()?.parse()?;
            args.name = Some(value.value());
            return Ok(());
        }
        if meta.path.is_ident("scope") {
            let value: LitStr = meta.value()?.parse()?;
            args.scope = Some(value.value());
            return Ok(());
        }
        if meta.path.is_ident("lazy") {
            let value: LitBool = meta.value()?.parse()?;
            args.lazy = Some(value.value());
            return Ok(());
        }
        if meta.path.is_ident("deps") {
            let expr: Expr = meta.value()?.parse()?;
            match expr {
                Expr::Array(ExprArray { elems, .. }) => {
                    for elem in elems {
                        match elem {
                            Expr::Lit(ExprLit {
                                lit: Lit::Str(s), ..
                            }) => args.deps.push(s.value()),
                            _ => return Err(meta.error("deps must be string literals")),
                        }
                    }
                }
                Expr::Lit(ExprLit {
                    lit: Lit::Str(s), ..
                }) => {
                    args.deps.push(s.value());
                }
                _ => return Err(meta.error("deps must be string literals")),
            }
            return Ok(());
        }
        Err(meta.error("unsupported component attribute"))
    });
    parser.parse(attribute)?;
    Ok(args)
}

fn default_bean_name(ident: &syn::Ident) -> String {
    let raw = ident.to_string();
    let mut chars = raw.chars();
    match chars.next() {
        Some(first) => format!("{}{}", first.to_lowercase(), chars.collect::<String>()),
        None => raw,
    }
}

fn extract_dependency_name(ty: &Type) -> Option<String> {
    match ty {
        Type::Path(path) => {
            let segment = path.path.segments.first()?;
            let ident = segment.ident.to_string();
            if ident == "Option" || ident == "Box" {
                if let PathArguments::AngleBracketed(args) = &segment.arguments {
                    let inner = args.args.first()?;
                    if let GenericArgument::Type(inner_ty) = inner {
                        return extract_dependency_name(inner_ty);
                    }
                }
            }
            if is_primitive_type(&ident) {
                return None;
            }
            Some(lowercase_first(&ident))
        }
        _ => None,
    }
}

fn is_primitive_type(ident: &str) -> bool {
    matches!(
        ident,
        "i8" | "i16"
            | "i32"
            | "i64"
            | "i128"
            | "isize"
            | "u8"
            | "u16"
            | "u32"
            | "u64"
            | "u128"
            | "usize"
            | "f32"
            | "f64"
            | "bool"
            | "String"
    )
}

fn lowercase_first(raw: &str) -> String {
    let mut chars = raw.chars();
    match chars.next() {
        Some(first) => format!("{}{}", first.to_lowercase(), chars.collect::<String>()),
        None => raw.to_string(),
    }
}

/// 同时剥离 struct 字段上的 #[autowired] 和 struct 上的 #[Scope] / #[Lazy] / #[ConditionalOnProperty]
fn strip_helper_attrs(mut input: ItemStruct) -> ItemStruct {
    // 剥离 struct-level helper attrs
    input.attrs.retain(|attr| {
        !path_has_ident(attr.path(), "Scope")
            && !path_has_ident(attr.path(), "scope")
            && !path_has_ident(attr.path(), "Lazy")
            && !path_has_ident(attr.path(), "lazy")
            && !path_has_ident(attr.path(), "ConditionalOnProperty")
    });
    // 剥离字段上的 #[autowired]
    if let Fields::Named(ref mut fields) = input.fields {
        for field in fields.named.iter_mut() {
            field.attrs.retain(|attr| {
                !path_has_ident(attr.path(), "autowired")
                    && !path_has_ident(attr.path(), "Value")
                    && !path_has_ident(attr.path(), "value")
            });
        }
    }
    input
}

/// 从 struct-level attrs 中读取 #[Scope("...")]
fn extract_scope_attr(attrs: &[Attribute]) -> Option<String> {
    for attr in attrs {
        if path_has_ident(attr.path(), "Scope") || path_has_ident(attr.path(), "scope") {
            // 支持两种语法: #[Scope("prototype")] 和 #[Scope = "prototype"]
            if let Ok(lit) = attr.parse_args::<LitStr>() {
                return Some(lit.value());
            }
        }
    }
    None
}

/// 从 struct-level attrs 中读取 #[Lazy] / #[Lazy(true)] / #[Lazy(false)]
fn extract_lazy_attr(attrs: &[Attribute]) -> Option<bool> {
    for attr in attrs {
        if path_has_ident(attr.path(), "Lazy") || path_has_ident(attr.path(), "lazy") {
            // #[Lazy] 无参数 => true，#[Lazy(false)] => false
            if let Ok(lit) = attr.parse_args::<LitBool>() {
                return Some(lit.value());
            }
            return Some(true);
        }
    }
    None
}

/// 从 struct-level attrs 中读取 #[ConditionalOnProperty("key", having = "value")]
/// 支持两种语法：
///   1. `#[ConditionalOnProperty("key")]`            → 匹配值默认为 "true"
///   2. `#[ConditionalOnProperty("key", having = "value")]` → 匹配指定值
fn extract_conditional_attr(attrs: &[Attribute]) -> Option<(String, String)> {
    for attr in attrs {
        if path_has_ident(attr.path(), "ConditionalOnProperty") {
            let mut key = String::new();
            let mut having = "true".to_string();

            // 用自定义解析器: 先读第一个 LitStr（key），再读可选的 , having = "..."
            let parser = |input: syn::parse::ParseStream| -> syn::Result<()> {
                // 第一个位置参数: property key
                let lit: LitStr = input.parse()?;
                key = lit.value();
                // 可选的命名参数
                while input.peek(syn::Token![,]) {
                    let _comma: syn::Token![,] = input.parse()?;
                    if input.is_empty() {
                        break;
                    }
                    let ident: Ident = input.parse()?;
                    let _eq: syn::Token![=] = input.parse()?;
                    if ident == "having" {
                        let val: LitStr = input.parse()?;
                        having = val.value();
                    }
                }
                Ok(())
            };
            let _ = attr.parse_args_with(parser);

            if !key.is_empty() {
                return Some((key, having));
            }
        }
    }
    None
}
