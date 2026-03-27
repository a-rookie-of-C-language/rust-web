pub trait BeanPostProcessor {
    fn post_process_before_initialization(&self, bean_name: &str, bean: &mut dyn std::any::Any);
    fn post_process_after_initialization(&self, bean_name: &str, bean: &mut dyn std::any::Any);
    fn order(&self) -> i32 {
        0
    }
}
pub struct DefaultBeanPostProcessor {}

impl BeanPostProcessor for DefaultBeanPostProcessor {
    fn post_process_before_initialization(&self, bean_name: &str, _bean: &mut dyn std::any::Any) {
        println!(
            "DefaultBeanPostProcessor: Before Initialization of bean '{}'",
            bean_name
        );
    }

    fn post_process_after_initialization(&self, bean_name: &str, _bean: &mut dyn std::any::Any) {
        println!(
            "DefaultBeanPostProcessor: After Initialization of bean '{}'",
            bean_name
        );
    }
}
