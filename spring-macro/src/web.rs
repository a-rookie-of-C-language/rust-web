use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{parse_macro_input, FnArg, Ident, ItemFn, LitStr, Type};

// ─────────────────────────────────────────────────────────────────────────────
// 公共入口（按 HTTP 方法区分）
// ─────────────────────────────────────────────────────────────────────────────

pub fn get_mapping_impl(attr: TokenStream, item: TokenStream) -> TokenStream {
    mapping_impl("GET", attr, item)
}
pub fn post_mapping_impl(attr: TokenStream, item: TokenStream) -> TokenStream {
    mapping_impl("POST", attr, item)
}
pub fn put_mapping_impl(attr: TokenStream, item: TokenStream) -> TokenStream {
    mapping_impl("PUT", attr, item)
}
pub fn delete_mapping_impl(attr: TokenStream, item: TokenStream) -> TokenStream {
    mapping_impl("DELETE", attr, item)
}
pub fn patch_mapping_impl(attr: TokenStream, item: TokenStream) -> TokenStream {
    mapping_impl("PATCH", attr, item)
}

/// 透传：仅作标记用，真正的注册由 GetMapping 等宏完成
pub fn rest_controller_impl(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item // 透传，#[Component] 处理 IoC 注册
}

// ─────────────────────────────────────────────────────────────────────────────
// 核心实现
// ─────────────────────────────────────────────────────────────────────────────

/// 通用路由宏实现。
///
/// 支持两种 handler 签名：
///
/// 1. **Plain handler** — 无 bean 注入:
///    ```ignore
///    #[GetMapping("/hello")]
///    fn hello(req: &HttpRequest) -> HttpResponse { ... }
///    ```
///
/// 2. **Bean handler** — 从 IoC 容器注入第一个参数所对应的 bean:
///    ```ignore
///    #[GetMapping("/users")]
///    fn list_users(ctrl: &UserController, req: &HttpRequest) -> HttpResponse { ... }
///    ```
///    宏自动从 `UserController` 推导 bean 名称为 `"userController"`。
fn mapping_impl(method: &str, attr: TokenStream, item: TokenStream) -> TokenStream {
    // 1. 解析路径字面量  e.g. "/users/{id}"
    let path_lit = parse_macro_input!(attr as LitStr);
    let path_str = path_lit.value();

    // 2. 解析被注解的函数
    let func = parse_macro_input!(item as ItemFn);
    let func_name = &func.sig.ident;
    let inputs = &func.sig.inputs;

    // 3. 生成 HttpMethod token
    let method_ident = Ident::new(method, Span::call_site());

    // 4. 根据参数个数判断 handler 类型
    let register = match inputs.len() {
        // ── Plain handler: fn handler(req: &HttpRequest) -> HttpResponse ──
        1 => {
            quote! {
                inventory::submit! {
                    spring_boot::web::RouteRegistration {
                        method:  spring_boot::web::HttpMethod::#method_ident,
                        path:    #path_str,
                        handler: spring_boot::web::Handler::Plain(#func_name),
                    }
                }
            }
        }
        // ── Bean handler: fn handler(ctrl: &ControllerType, req: &HttpRequest) ──
        2 => {
            // 提取第一个参数的类型 Ident（去掉 & 引用）
            let bean_type_ident = match extract_ref_type_ident(&inputs[0]) {
                Some(id) => id,
                None => {
                    return syn::Error::new_spanned(
                        &inputs[0],
                        "first parameter must be a shared reference, e.g. `ctrl: &UserController`",
                    )
                    .to_compile_error()
                    .into();
                }
            };

            // 派生 bean 名称：首字母小写
            let bean_name = camel_to_bean_name(&bean_type_ident.to_string());
            let bean_name_lit = LitStr::new(&bean_name, Span::call_site());

            // 生成包装函数，负责 downcast + 实际调用
            let wrapper_name = Ident::new(
                &format!("__spring_web_handler_{}", func_name),
                Span::call_site(),
            );

            quote! {
                fn #wrapper_name(
                    req:  &spring_boot::web::HttpRequest,
                    bean: &(dyn std::any::Any + Send + Sync),
                ) -> spring_boot::web::HttpResponse {
                    if let Some(ctrl) = bean.downcast_ref::<#bean_type_ident>() {
                        #func_name(ctrl, req)
                    } else {
                        spring_boot::web::HttpResponse::internal_error().json(concat!(
                            "{\"error\":\"bean type mismatch: ",
                            #bean_name,
                            "\"}"
                        ))
                    }
                }

                inventory::submit! {
                    spring_boot::web::RouteRegistration {
                        method:  spring_boot::web::HttpMethod::#method_ident,
                        path:    #path_str,
                        handler: spring_boot::web::Handler::WithBean {
                            bean_name: #bean_name_lit,
                            f:         #wrapper_name,
                        },
                    }
                }
            }
        }
        _ => {
            return syn::Error::new_spanned(
                &func.sig,
                "#[GetMapping] handler must have exactly 1 or 2 parameters: \
                 `(req: &HttpRequest)` or `(ctrl: &Controller, req: &HttpRequest)`",
            )
            .to_compile_error()
            .into();
        }
    };

    // 保留原函数，附加注册代码
    let expanded = quote! {
        #func
        #register
    };
    expanded.into()
}

// ─────────────────────────────────────────────────────────────────────────────
// 工具函数
// ─────────────────────────────────────────────────────────────────────────────

/// 从 `ctrl: &SomeType` 形式的 FnArg 中提取 `SomeType` 的 Ident。
fn extract_ref_type_ident(arg: &FnArg) -> Option<Ident> {
    if let FnArg::Typed(pat_type) = arg {
        if let Type::Reference(type_ref) = &*pat_type.ty {
            if let Type::Path(type_path) = &*type_ref.elem {
                return type_path.path.get_ident().cloned();
            }
        }
    }
    None
}

/// `UserController` → `"userController"`（首字母小写）
fn camel_to_bean_name(type_name: &str) -> String {
    let mut chars = type_name.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_lowercase().to_string() + chars.as_str(),
    }
}
