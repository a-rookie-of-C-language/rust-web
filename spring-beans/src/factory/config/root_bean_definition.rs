use super::bean_definition::{BeanDefinition, BeanScope};
use spring_macro::{all_args_constructor, data};
use std::any::{Any, TypeId};

type ResolvedDeps = std::collections::HashMap<String, Box<dyn Any>>;
type EnvMap = std::collections::HashMap<String, String>;
type Supplier = dyn Fn(&ResolvedDeps, &EnvMap) -> Box<dyn Any>;

#[data]
#[all_args_constructor]
pub struct RootBeanDefinition {
    name: String,
    type_id: TypeId,
    scope: BeanScope,
    is_lazy: bool,
    dependencies: Vec<String>,
    supplier: Box<Supplier>,
    /// Optional `(property_key, expected_value)` condition.
    /// Set by `#[ConditionalOnProperty("key", having = "value")]`.
    condition: Option<(String, String)>,
}

impl BeanDefinition for RootBeanDefinition {
    fn get_bean_class_name(&self) -> &str {
        &self.name
    }

    fn set_scope(&mut self, scope: BeanScope) {
        self.scope = scope;
    }

    fn get_scope(&self) -> BeanScope {
        self.scope
    }

    fn is_lazy_init(&self) -> bool {
        self.is_lazy
    }

    fn set_lazy_init(&mut self, lazy: bool) {
        self.is_lazy = lazy;
    }

    fn get_type_id(&self) -> TypeId {
        self.type_id
    }

    fn has_annotation(&self, annotation: &str) -> bool {
        annotation == "RootBeanDefinition"
    }

    fn create_instance(
        &self,
        resolved_deps: &std::collections::HashMap<String, Box<dyn Any>>,
        env: &std::collections::HashMap<String, String>,
    ) -> Box<dyn Any> {
        (self.supplier)(resolved_deps, env)
    }

    fn get_dependencies(&self) -> Vec<String> {
        self.dependencies.clone()
    }

    fn get_condition(&self) -> Option<(&str, &str)> {
        self.condition
            .as_ref()
            .map(|(k, v)| (k.as_str(), v.as_str()))
    }
}
