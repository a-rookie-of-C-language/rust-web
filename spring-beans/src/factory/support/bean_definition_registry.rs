use crate::factory::config::bean_definition::BeanDefinition;

pub trait BeanDefinitionRegistry {
    fn register_bean_definition(
        &mut self,
        bean_name: &str,
        bean_definition: Box<dyn BeanDefinition>,
    );
    fn remove_bean_definition(&mut self, bean_name: &str);
    fn contains_bean_definition(&self, bean_name: &str) -> bool;
    fn get_bean_definition(&self, bean_name: &str) -> Option<&dyn BeanDefinition>;
    fn get_bean_definition_names(&self) -> Vec<String>;
    fn get_bean_definition_count(&self) -> usize;
    fn is_bean_name_in_use(&self, bean_name: &str) -> bool;
}
