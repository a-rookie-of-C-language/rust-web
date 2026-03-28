use crate::factory::bean_factory::SharedBean;
use crate::factory::BeanFactory;
use std::any::TypeId;

pub trait ListableBeanFactory: BeanFactory {
    fn contains_bean_definition(&self, _name: &str) -> bool;
    fn get_bean_definition_count(&self) -> usize {
        0
    }
    fn get_bean_definition_names(&self) -> Vec<String>;
    fn get_bean_names_for_type<T>(&self, _type_id: TypeId) -> Vec<String>;
    fn get_beans_of_type<T: 'static>(&self) -> Vec<SharedBean>;
    fn get_bean_definition_names_for_annotation(&self, _annotation: &str) -> Vec<String>;
}
