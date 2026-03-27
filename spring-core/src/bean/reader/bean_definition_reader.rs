use crate::registry::BeanDefinitionRegistry;

pub trait BeanDefinitionReader {
    fn load_bean_definitions(&mut self, location: &str);
    fn get_registry(&self) -> &dyn BeanDefinitionRegistry;
    // ResourceLoader 暂时注释，后续实现
    // fn get_resource_loader(&self) -> &dyn ResourceLoader;
    fn load_bean_definitions_from_resource(&mut self, resource: &dyn std::any::Any);
}
