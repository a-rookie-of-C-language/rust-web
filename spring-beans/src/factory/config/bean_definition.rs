pub type SharedBean = std::sync::Arc<dyn std::any::Any + Send + Sync>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BeanScope {
    Singleton,
    Prototype,
}

pub trait BeanDefinition: Send + Sync {
    fn get_bean_class_name(&self) -> &str;
    fn set_scope(&mut self, scope: BeanScope);
    fn get_scope(&self) -> BeanScope;
    fn is_lazy_init(&self) -> bool;
    fn set_lazy_init(&mut self, lazy: bool);
    fn get_type_id(&self) -> std::any::TypeId;
    fn has_annotation(&self, annotation: &str) -> bool;
    fn create_instance(
        &self,
        resolved_deps: &std::collections::HashMap<String, SharedBean>,
        env: &std::collections::HashMap<String, String>,
    ) -> SharedBean;
    fn get_dependencies(&self) -> Vec<String>;

    /// Returns the `(property_key, expected_value)` condition for this bean,
    /// or `None` if the bean is unconditional.
    fn get_condition(&self) -> Option<(&str, &str)> {
        None
    }
}
