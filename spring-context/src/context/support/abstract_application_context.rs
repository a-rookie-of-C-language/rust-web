use crate::context::application_context::ApplicationContext;
use crate::context::configurable_application_context::ConfigurableApplicationContext;
use crate::context::lifecycle::Lifecycle;
use spring_beans::factory::config::{BeanDefinition, BeanScope, ConfigurableBeanFactory};
use spring_beans::factory::{BeanDefinitionRegistry, BeanFactory, DefaultListableBeanFactory};
use spring_macro::data;

#[data]
pub struct AbstractApplicationContext {
    bean_factory: DefaultListableBeanFactory,
}

impl ConfigurableApplicationContext for AbstractApplicationContext {
    fn refresh(&mut self) {
        let names = spring_beans::factory::BeanDefinitionRegistry::get_bean_definition_names(&self.bean_factory);
        for name in names {
            if let Some(definition) = self.bean_factory.get_bean_definition(&name) {
                if !definition.is_lazy_init() && definition.get_scope() == BeanScope::Singleton {
                    self.bean_factory.do_create_bean(&name);
                }
            }
        }
    }

    fn close(&mut self) {
        self.bean_factory.destroy_singletons();
    }

    fn is_active(&self) -> bool {
        true
    }
}

impl Lifecycle for AbstractApplicationContext {
    fn start(&mut self) {}

    fn stop(&mut self) {}

    fn is_running(&self) -> bool {
        true
    }
}

impl ApplicationContext for AbstractApplicationContext {
    fn contains_bean(&self, name: &str) -> bool {
        self.bean_factory.contains_bean(name)
    }

    fn do_create_bean(&self, name: &str) -> Option<crate::context::application_context::SharedBean> {
        self.bean_factory.do_create_bean(name)
    }

    fn get_bean(&self, name: &str) -> Option<crate::context::application_context::SharedBean> {
        self.bean_factory.get_bean(name)
    }

    fn is_singleton(&self, name: &str) -> bool {
        self.bean_factory.is_singleton(name)
    }
}

impl BeanDefinitionRegistry for AbstractApplicationContext {
    fn register_bean_definition(&mut self, name: &str, bean_definition: Box<dyn BeanDefinition>) {
        self.bean_factory
            .register_bean_definition(name, bean_definition);
    }

    fn remove_bean_definition(&mut self, bean_name: &str) {
        self.bean_factory.remove_bean_definition(bean_name);
    }

    fn contains_bean_definition(&self, bean_name: &str) -> bool {
        self.bean_factory.contains_bean_definition(bean_name)
    }

    fn get_bean_definition(&self, bean_name: &str) -> Option<&dyn BeanDefinition> {
        self.bean_factory.get_bean_definition(bean_name)
    }

    fn get_bean_definition_names(&self) -> Vec<String> {
        spring_beans::factory::BeanDefinitionRegistry::get_bean_definition_names(&self.bean_factory)
    }

    fn get_bean_definition_count(&self) -> usize {
        self.bean_factory.get_bean_definition_count()
    }

    fn is_bean_name_in_use(&self, bean_name: &str) -> bool {
        self.bean_factory.is_bean_name_in_use(bean_name)
    }
}

impl Default for AbstractApplicationContext {
    fn default() -> Self {
        Self {
            bean_factory: DefaultListableBeanFactory::new(),
        }
    }
}
impl AbstractApplicationContext {
    pub fn register_post_processor(
        &mut self,
        processor: Box<dyn spring_beans::bean::bean_post_processor::BeanPostProcessor>,
    ) {
        self.bean_factory.register_post_processor(processor);
    }
    pub fn set_environment(&mut self, environment: spring_beans::env::Environment) {
        self.bean_factory.set_environment(environment);
    }
}
