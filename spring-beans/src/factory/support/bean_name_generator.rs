pub trait BeanNameGenerator {
    fn generate_bean_name(
        &self,
        bean_definition: &dyn std::any::Any,
        registry: &dyn std::any::Any,
    ) -> String;
}
