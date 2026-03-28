use crate::factory::BeanFactory;
use crate::factory::bean_factory::SharedBean;

pub trait ConfigurableBeanFactory: BeanFactory {
    fn register_singleton(&mut self, _bean_name: &str, _singleton_object: SharedBean) {}
    fn destroy_singleton(&mut self, _bean_name: &str) {}
    fn destroy_singletons(&mut self) {}
}
