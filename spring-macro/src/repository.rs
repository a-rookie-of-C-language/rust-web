use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{parse::Parse, parse::ParseStream, parse_macro_input, Ident, ItemStruct, LitStr, Token};

// ─── Attribute 解析 ───────────────────────────────────────────────────────────
// 支持两种写法：
//   #[Repository(User)]
//   #[Repository(entity = "User")]

struct RepositoryArgs {
    entity: Ident,
}

impl Parse for RepositoryArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        // entity = "TypeName"
        if input.peek(Ident) && {
            let fork = input.fork();
            let _: Ident = fork.parse()?;
            fork.peek(Token![=])
        } {
            let key: Ident = input.parse()?;
            if key != "entity" {
                return Err(syn::Error::new_spanned(key, "expected `entity`"));
            }
            let _: Token![=] = input.parse()?;
            let lit: LitStr = input.parse()?;
            let entity = Ident::new(&lit.value(), lit.span());
            return Ok(RepositoryArgs { entity });
        }
        // 裸类型名: #[Repository(User)]
        let entity: Ident = input.parse()?;
        Ok(RepositoryArgs { entity })
    }
}

// ─── 宏实现 ──────────────────────────────────────────────────────────────────

pub fn repository_impl(attribute: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attribute as RepositoryArgs);
    let input = parse_macro_input!(item as ItemStruct);

    let struct_ident = &input.ident;
    let entity_ident = &args.entity;

    // bean 名称：首字母小写
    let name = default_bean_name(struct_ident);
    let name_lit = LitStr::new(&name, Span::call_site());

    let expanded = quote! {
        // ── 生成的结构体 ──────────────────────────────────────────
        pub struct #struct_ident {
            inner: spring_boot::data::InMemoryRepository<#entity_ident>,
        }

        impl ::std::default::Default for #struct_ident {
            fn default() -> Self {
                Self { inner: spring_boot::data::InMemoryRepository::new() }
            }
        }

        // ── CRUD 委托方法 ─────────────────────────────────────────
        impl #struct_ident {
            /// 保存实体，返回自动分配的 u64 主键
            pub fn save(&self, entity: #entity_ident) -> u64 {
                spring_boot::data::Repository::save(&self.inner, entity)
            }

            /// 按 id 更新；若不存在则返回 false
            pub fn update(&self, id: u64, entity: #entity_ident) -> bool {
                spring_boot::data::Repository::update(&self.inner, id, entity)
            }

            /// 按 id 查询，通过闭包返回结果（借用安全）
            pub fn find_by_id<R, F: FnOnce(Option<&#entity_ident>) -> R>(&self, id: u64, f: F) -> R {
                spring_boot::data::Repository::find_by_id(&self.inner, id, f)
            }

            /// 将全部记录克隆为 Vec<(u64, T)>（需要 T: Clone）
            pub fn find_all_cloned(&self) -> Vec<(u64, #entity_ident)>
            where
                #entity_ident: Clone,
            {
                spring_boot::data::Repository::find_all_cloned(&self.inner)
            }

            /// 遍历所有记录（不克隆）
            pub fn for_each<F: FnMut(u64, &#entity_ident)>(&self, f: F) {
                spring_boot::data::Repository::for_each(&self.inner, f)
            }

            /// 按 id 删除
            pub fn delete_by_id(&self, id: u64) -> bool {
                spring_boot::data::Repository::delete_by_id(&self.inner, id)
            }

            /// 清空所有记录并重置主键计数
            pub fn delete_all(&self) {
                spring_boot::data::Repository::delete_all(&self.inner)
            }

            /// 记录总数
            pub fn count(&self) -> usize {
                spring_boot::data::Repository::count(&self.inner)
            }

            /// 是否存在指定 id
            pub fn exists_by_id(&self, id: u64) -> bool {
                spring_boot::data::Repository::exists_by_id(&self.inner, id)
            }
        }

        // ── IoC bean 注册 ─────────────────────────────────────────
        impl #struct_ident {
            pub fn bean_name() -> &'static str {
                #name_lit
            }

            pub fn bean_definition() -> spring_beans::factory::config::RootBeanDefinition {
                spring_beans::factory::config::RootBeanDefinition::new(
                    #name_lit.to_string(),
                    std::any::TypeId::of::<#struct_ident>(),
                    spring_beans::factory::config::BeanScope::Singleton,
                    false,
                    vec![],
                    Box::new(|_resolved_deps: &std::collections::HashMap<String, std::sync::Arc<dyn std::any::Any + Send + Sync>>,
                               _env: &std::collections::HashMap<String, String>| {
                        std::sync::Arc::new(#struct_ident::default()) as std::sync::Arc<dyn std::any::Any + Send + Sync>
                    }),
                    None,
                )
            }
        }

        inventory::submit! {
            spring_beans::registry::BeanRegistration {
                definition: || #struct_ident::bean_definition(),
            }
        }
    };

    expanded.into()
}

// ─── helpers ─────────────────────────────────────────────────────────────────

fn default_bean_name(ident: &Ident) -> String {
    let s = ident.to_string();
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_lowercase().to_string() + chars.as_str(),
    }
}
