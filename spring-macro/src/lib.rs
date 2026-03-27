use proc_macro::TokenStream;

mod accessors;
mod all_args_constructor;
mod aop_methods;
mod aspect;
mod bean;
mod component;
mod data;
mod getter;
mod no_arg_constructor;
mod repository;
mod setter;
mod value;
mod web;
#[proc_macro_attribute]
pub fn component(attribute: TokenStream, item: TokenStream) -> TokenStream {
    component::component_impl(attribute, item)
}

/// derive macro 内部别名（保持向后兼容）
#[proc_macro_derive(ComponentDerive, attributes(autowired))]
pub fn component_derive(item: TokenStream) -> TokenStream {
    component::component_derive_impl(item)
}

/// #[Component] attribute macro —— Spring 风格的主入口，自动处理 #[autowired] 字段注入
#[proc_macro_attribute]
#[allow(non_snake_case)]
pub fn Component(attribute: TokenStream, item: TokenStream) -> TokenStream {
    component::component_impl(attribute, item)
}

/// #[Scope("prototype")] / #[Scope("singleton")] —— 附加在 #[Component] struct 上，指定 bean 作用域
/// 本宏仅作 helper attribute 使用，真正逻辑由 #[Component] 处理。
#[proc_macro_attribute]
#[allow(non_snake_case)]
pub fn Scope(_attribute: TokenStream, item: TokenStream) -> TokenStream {
    item // 透传，内容由 #[Component] 处理
}

/// #[Lazy] / #[Lazy(false)] —— 附加在 #[Component] struct 上，指定 bean 是否延迟初始化
#[proc_macro_attribute]
#[allow(non_snake_case)]
pub fn Lazy(_attribute: TokenStream, item: TokenStream) -> TokenStream {
    item // 透传，内容由 #[Component] 处理
}

/// #[Bean] —— 方法级别注解，类似 Java @Bean。标注在函数上，函数返回值就是 bean 实例。
/// 支持: #[Bean] / #[Bean(name="foo")] / #[Bean(scope="prototype")] / #[Bean(lazy=true)]
#[proc_macro_attribute]
#[allow(non_snake_case)]
pub fn Bean(attribute: TokenStream, item: TokenStream) -> TokenStream {
    bean::bean_impl(attribute, item)
}

/// #[Value("${key:default}")] —— 字段级注解，从 Environment 注入配置值。
/// 本宏仅作 helper attribute 使用，真正逻辑由 #[Component] 处理。
#[proc_macro_attribute]
#[allow(non_snake_case)]
pub fn Value(attribute: TokenStream, item: TokenStream) -> TokenStream {
    value::value_impl(attribute, item)
}

#[proc_macro_attribute]
pub fn data(attribute: TokenStream, item: TokenStream) -> TokenStream {
    data::data_impl(attribute, item)
}

#[proc_macro_attribute]
pub fn getter(attribute: TokenStream, item: TokenStream) -> TokenStream {
    getter::getter_impl(attribute, item)
}

#[proc_macro_attribute]
pub fn setter(attribute: TokenStream, item: TokenStream) -> TokenStream {
    setter::setter_impl(attribute, item)
}

#[proc_macro_attribute]
pub fn no_arg_constructor(attribute: TokenStream, item: TokenStream) -> TokenStream {
    no_arg_constructor::no_arg_constructor_impl(attribute, item)
}

#[proc_macro_attribute]
pub fn all_args_constructor(attribute: TokenStream, item: TokenStream) -> TokenStream {
    all_args_constructor::all_args_constructor_impl(attribute, item)
}

/// #[Aspect] —— Marks a struct as an aspect container (pass-through).
#[proc_macro_attribute]
#[allow(non_snake_case)]
pub fn Aspect(attribute: TokenStream, item: TokenStream) -> TokenStream {
    aspect::aspect_impl(attribute, item)
}

/// #[Before("beanName::methodName")] —— registers a Before advice.
#[proc_macro_attribute]
#[allow(non_snake_case)]
pub fn Before(attribute: TokenStream, item: TokenStream) -> TokenStream {
    aspect::before_impl(attribute, item)
}

/// #[After("beanName::methodName")] —— registers an After advice.
#[proc_macro_attribute]
#[allow(non_snake_case)]
pub fn After(attribute: TokenStream, item: TokenStream) -> TokenStream {
    aspect::after_impl(attribute, item)
}

/// #[Around("beanName::methodName")] —— registers an Around (before+after) advice.
#[proc_macro_attribute]
#[allow(non_snake_case)]
pub fn Around(attribute: TokenStream, item: TokenStream) -> TokenStream {
    aspect::around_impl(attribute, item)
}

/// #[AopMethods] —— Apply to an `impl` block to automatically weave AOP into
/// every `pub fn` that takes `&self` / `&mut self`.
#[proc_macro_attribute]
#[allow(non_snake_case)]
pub fn AopMethods(attribute: TokenStream, item: TokenStream) -> TokenStream {
    aop_methods::aop_methods_impl(attribute, item)
}

/// #[ConditionalOnProperty("key", having = "value")] —— the bean is only registered
/// when `application.properties` contains `key=value`.
///
/// `having` is optional and defaults to `"true"` when omitted.
/// This is a helper attribute; the real logic is handled by `#[Component]`.
#[proc_macro_attribute]
#[allow(non_snake_case)]
pub fn ConditionalOnProperty(_attribute: TokenStream, item: TokenStream) -> TokenStream {
    item
}

/// #[Repository(User)] / #[Repository(entity = "User")]
/// 标注在空 struct 上，自动生成内存 CRUD 方法并注册为 IoC bean。
#[proc_macro_attribute]
#[allow(non_snake_case)]
pub fn Repository(attribute: TokenStream, item: TokenStream) -> TokenStream {
    repository::repository_impl(attribute, item)
}

/// #[RestController] —— 标记 struct 为 REST 控制器（透传，实际注册由 #[Component] 处理）
#[proc_macro_attribute]
#[allow(non_snake_case)]
pub fn RestController(attribute: TokenStream, item: TokenStream) -> TokenStream {
    web::rest_controller_impl(attribute, item)
}

/// #[GetMapping("/path")] —— 注册 GET 路由
#[proc_macro_attribute]
#[allow(non_snake_case)]
pub fn GetMapping(attribute: TokenStream, item: TokenStream) -> TokenStream {
    web::get_mapping_impl(attribute, item)
}

/// #[PostMapping("/path")] —— 注册 POST 路由
#[proc_macro_attribute]
#[allow(non_snake_case)]
pub fn PostMapping(attribute: TokenStream, item: TokenStream) -> TokenStream {
    web::post_mapping_impl(attribute, item)
}

/// #[PutMapping("/path")] —— 注册 PUT 路由
#[proc_macro_attribute]
#[allow(non_snake_case)]
pub fn PutMapping(attribute: TokenStream, item: TokenStream) -> TokenStream {
    web::put_mapping_impl(attribute, item)
}

/// #[DeleteMapping("/path")] —— 注册 DELETE 路由
#[proc_macro_attribute]
#[allow(non_snake_case)]
pub fn DeleteMapping(attribute: TokenStream, item: TokenStream) -> TokenStream {
    web::delete_mapping_impl(attribute, item)
}

/// #[PatchMapping("/path")] —— 注册 PATCH 路由
#[proc_macro_attribute]
#[allow(non_snake_case)]
pub fn PatchMapping(attribute: TokenStream, item: TokenStream) -> TokenStream {
    web::patch_mapping_impl(attribute, item)
}
