pub type SharedBean = std::sync::Arc<dyn std::any::Any + Send + Sync>;

pub trait BeanFactory: Send + Sync {
    fn get_bean(&self, name: &str) -> Option<SharedBean>;
    fn is_singleton(&self, name: &str) -> bool;
    fn contains_bean(&self, name: &str) -> bool;
    fn do_create_bean(&self, name: &str) -> Option<SharedBean>;
}
