use spring_beans::factory::support::BeanDefinitionRegistry;
use spring_beans::factory::BeanDefinition;
use spring_beans::factory::DefaultListableBeanFactory;
pub struct GenericApplicationContext {
    bean_factory: DefaultListableBeanFactory,
}

impl BeanDefinitionRegistry for GenericApplicationContext {
    fn register_bean_definition(&mut self, name: &str, bean_definition: Box<dyn BeanDefinition>) {
        self.bean_factory
            .register_bean_definition(name, bean_definition);
    }

    fn contains_bean_definition(&self, bean_name: &str) -> bool {
        self.bean_factory.contains_bean_definition(bean_name)
    }

    fn get_bean_definition(
        &self,
        bean_name: &str,
    ) -> Option<&dyn spring_beans::factory::BeanDefinition> {
        self.bean_factory.get_bean_definition(bean_name)
    }

    fn get_bean_definition_count(&self) -> usize {
        self.bean_factory.get_bean_definition_count()
    }

    fn get_bean_definition_names(&self) -> &Vec<String> {
        self.bean_factory.get_bean_definition_names()
    }

    fn is_bean_name_in_use(&self, bean_name: &str) -> bool {
        self.bean_factory.is_bean_name_in_use(bean_name)
    }

    fn remove_bean_definition(&mut self, bean_name: &str) {
        self.bean_factory.remove_bean_definition(bean_name);
    }
}
