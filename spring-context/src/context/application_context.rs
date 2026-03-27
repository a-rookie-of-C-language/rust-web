pub trait ApplicationContext {
    fn get_bean(&self, name: &str) -> Option<&dyn std::any::Any>;
    fn is_singleton(&self, name: &str) -> bool;
    fn contains_bean(&self, name: &str) -> bool;
    fn do_create_bean(&mut self, name: &str) -> Option<&dyn std::any::Any>;
}
