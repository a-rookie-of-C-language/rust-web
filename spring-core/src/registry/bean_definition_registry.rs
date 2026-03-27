use spring_beans::factory::BeanDefinition;
pub trait BeanDefinitionRegistry {
    fn register_bean_definition(&mut self, name: &str, bean_definition: Box<dyn BeanDefinition>);
    fn remove_bean_definition(&mut self, name: &str);
    fn contains_bean_definition(&self, name: &str) -> bool;
    fn get_bean_definition(&self, name: &str) -> Option<&dyn BeanDefinition>;
    fn get_bean_definition_names(&self) -> Vec<String>;
}
