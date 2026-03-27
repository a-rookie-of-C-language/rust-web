use crate::bean::bean_post_processor_register::BeanPostProcessorRegistry;
use crate::env::Environment;
use crate::factory::config::{BeanDefinition, BeanScope, ConfigurableBeanFactory};
use crate::factory::listable_bean_factory::ListableBeanFactory;
use crate::factory::BeanDefinitionRegistry;
use crate::factory::BeanFactory;
use spring_macro::data;
use std::any::Any;
use std::collections::{HashMap, HashSet};
use std::panic::{catch_unwind, AssertUnwindSafe};

#[data]
pub struct DefaultListableBeanFactory {
    bean_definition_map: HashMap<String, Box<dyn BeanDefinition>>,
    bean_definition_names: Vec<String>,
    singleton_objects: HashMap<String, Box<dyn Any>>,
    early_singleton_objects: HashMap<String, Box<dyn Any>>,
    singleton_factories: HashMap<String, Box<dyn Fn() -> Box<dyn Any>>>,
    currently_in_creation: HashSet<String>,
    post_processor_registry: BeanPostProcessorRegistry,
    environment: Environment,
}

impl BeanDefinitionRegistry for DefaultListableBeanFactory {
    fn contains_bean_definition(&self, bean_name: &str) -> bool {
        self.bean_definition_map.contains_key(bean_name)
    }

    fn get_bean_definition(&self, bean_name: &str) -> Option<&dyn BeanDefinition> {
        self.bean_definition_map
            .get(bean_name)
            .map(|definition| definition.as_ref())
    }

    fn get_bean_definition_count(&self) -> usize {
        self.bean_definition_map.len()
    }

    fn get_bean_definition_names(&self) -> &Vec<String> {
        &self.bean_definition_names
    }

    fn is_bean_name_in_use(&self, bean_name: &str) -> bool {
        BeanDefinitionRegistry::contains_bean_definition(self, bean_name)
            || self.singleton_objects.contains_key(bean_name)
            || self.singleton_factories.contains_key(bean_name)
            || self.currently_in_creation.contains(bean_name)
    }

    fn register_bean_definition(
        &mut self,
        bean_name: &str,
        bean_definition: Box<dyn BeanDefinition>,
    ) {
        self.bean_definition_map
            .insert(bean_name.to_string(), bean_definition);
        self.bean_definition_names.push(bean_name.to_string());
    }

    fn remove_bean_definition(&mut self, bean_name: &str) {
        self.bean_definition_map.remove(bean_name);
        self.bean_definition_names.retain(|n| n != bean_name);
    }
}

impl BeanFactory for DefaultListableBeanFactory {
    fn get_bean(&self, name: &str) -> Option<&dyn Any> {
        self.singleton_objects.get(name).map(|boxed| boxed.as_ref())
    }

    fn is_singleton(&self, name: &str) -> bool {
        self.bean_definition_map
            .get(name)
            .map(|definition| definition.get_scope() == BeanScope::Singleton)
            .unwrap_or_else(|| self.singleton_objects.contains_key(name))
    }

    fn contains_bean(&self, name: &str) -> bool {
        self.bean_definition_map.contains_key(name) || self.singleton_objects.contains_key(name)
    }

    fn do_create_bean(&mut self, name: &str) -> Option<&dyn std::any::Any> {
        let (dependencies, scope) = {
            let Some(definition) = self.bean_definition_map.get(name) else {
                self.currently_in_creation.remove(name);
                return None;
            };
            (definition.get_dependencies(), definition.get_scope())
        };
        // 如果是 Singleton 且已创建，直接返回缓存
        if scope == BeanScope::Singleton && self.singleton_objects.contains_key(name) {
            return self.singleton_objects.get(name).map(|b| b.as_ref());
        }

        if self.currently_in_creation.contains(name) {
            eprintln!(
                "[spring-beans] circular dependency detected at bean '{}'",
                name
            );
            return None;
        }
        self.currently_in_creation.insert(name.to_string());
        // 先递归创建所有依赖
        for dep in &dependencies {
            if self.currently_in_creation.contains(dep) {
                eprintln!(
                    "[spring-beans] circular dependency detected: {} -> {}",
                    name, dep
                );
                self.currently_in_creation.remove(name);
                return None;
            }
            self.do_create_bean(dep);
        }

        if dependencies
            .iter()
            .any(|dep| !self.singleton_objects.contains_key(dep))
        {
            eprintln!(
                "[spring-beans] bean '{}' creation aborted: unresolved dependencies {:?}",
                name, dependencies
            );
            self.currently_in_creation.remove(name);
            return None;
        }
        // 从 singleton_objects 收集依赖快照（拷贝其中的引用信息，实际指针仔然存于容器）
        // 注意： supplier 闭包通过 &HashMap 读取依赖，调用 downcast_ref + clone 进行字段注入
        let mut instance: Box<dyn Any> = {
            // 临时收集依赖的引用 map，内容仅包含该 bean 声明的依赖项
            let deps_snapshot: std::collections::HashMap<String, Box<dyn Any>> = dependencies
                .iter()
                .filter_map(|dep_name| {
                    // 将依赖从容器暂时移出，供 supplier 闭包使用
                    self.singleton_objects
                        .remove(dep_name)
                        .map(|b| (dep_name.clone(), b))
                })
                .collect();
            let definition = self.bean_definition_map.get(name)?;
            let inst = match catch_unwind(AssertUnwindSafe(|| {
                definition.create_instance(&deps_snapshot, &self.environment.as_map())
            })) {
                Ok(instance) => instance,
                Err(_) => {
                    eprintln!(
                        "[spring-beans] bean '{}' creation aborted due to supplier panic",
                        name
                    );
                    for (dep_name, dep_bean) in deps_snapshot {
                        self.singleton_objects.insert(dep_name, dep_bean);
                    }
                    self.currently_in_creation.remove(name);
                    return None;
                }
            };
            // 将依赖放回容器
            for (dep_name, dep_bean) in deps_snapshot {
                self.singleton_objects.insert(dep_name, dep_bean);
            }
            inst
        };
        // BeanPostProcessor: before initialization
        self.post_processor_registry
            .apply_before_initialization(name, instance.as_mut());
        // BeanPostProcessor: after initialization
        self.post_processor_registry
            .apply_after_initialization(name, instance.as_mut());
        match scope {
            BeanScope::Singleton => {
                self.singleton_objects.insert(name.to_string(), instance);
                self.currently_in_creation.remove(name);
                self.singleton_objects.get(name).map(|b| b.as_ref())
            }
            BeanScope::Prototype => {
                // Prototype 不缓存，每次调用都创建新实例
                self.currently_in_creation.remove(name);
                None
            }
        }
    }
} // impl BeanFactory
impl ConfigurableBeanFactory for DefaultListableBeanFactory {
    fn register_singleton(&mut self, bean_name: &str, singleton_object: Box<dyn Any>) {
        self.singleton_objects
            .insert(bean_name.to_string(), singleton_object);
    }

    fn destroy_singleton(&mut self, bean_name: &str) {
        self.singleton_objects.remove(bean_name);
    }

    fn destroy_singletons(&mut self) {
        self.singleton_objects.clear();
        self.early_singleton_objects.clear();
        self.singleton_factories.clear();
        self.currently_in_creation.clear();
    }
}

impl ListableBeanFactory for DefaultListableBeanFactory {
    fn contains_bean_definition(&self, name: &str) -> bool {
        self.bean_definition_map.contains_key(name)
    }

    fn get_bean_definition_count(&self) -> usize {
        self.bean_definition_map.len()
    }

    fn get_bean_definition_names(&self) -> Vec<String> {
        self.bean_definition_names.clone()
    }

    fn get_bean_names_for_type<T>(&self, type_id: std::any::TypeId) -> Vec<String> {
        self.bean_definition_map
            .iter()
            .filter(|(_, bd)| bd.as_ref().get_type_id() == type_id)
            .map(|(name, _)| name.clone())
            .collect::<Vec<_>>()
    }

    fn get_beans_of_type<T: 'static>(&self) -> Vec<&T> {
        self.singleton_objects
            .iter()
            .filter_map(|(_, obj)| obj.as_ref().downcast_ref::<T>())
            .collect::<Vec<_>>()
    }

    fn get_bean_definition_names_for_annotation(&self, annotation: &str) -> Vec<String> {
        self.bean_definition_map
            .iter()
            .filter(|(_, bd)| bd.as_ref().has_annotation(annotation))
            .map(|(name, _)| name.clone())
            .collect::<Vec<_>>()
    }
}

impl DefaultListableBeanFactory {
    pub fn new() -> Self {
        Self {
            bean_definition_map: HashMap::new(),
            bean_definition_names: Vec::new(),
            singleton_objects: HashMap::new(),
            early_singleton_objects: HashMap::new(),
            singleton_factories: HashMap::new(),
            currently_in_creation: HashSet::new(),
            post_processor_registry: BeanPostProcessorRegistry::new(),
            environment: Environment::new(),
        }
    }

    pub fn register_post_processor(
        &mut self,
        processor: Box<dyn crate::bean::bean_post_processor::BeanPostProcessor>,
    ) {
        self.post_processor_registry.register(processor);
    }
}
impl Default for DefaultListableBeanFactory {
    fn default() -> Self {
        Self::new()
    }
}
