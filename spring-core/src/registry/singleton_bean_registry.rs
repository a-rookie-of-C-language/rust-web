pub trait SingletonBeanRegistry {
    fn resgiser_singleton(&mut self, bean_name: &str, singleton_object: Box<dyn std::any::Any>);
    fn get_singleton(&self, bean_name: &str) -> Option<&dyn std::any::Any>;
    fn contains_singleton(&self, bean_name: &str) -> bool;
    fn get_singleton_names(&self) -> Vec<String>;
}
