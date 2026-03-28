use crate::bean::bean_post_processor_register::BeanPostProcessorRegistry;
use crate::env::Environment;
use crate::factory::bean_factory::SharedBean;
use crate::factory::config::{BeanDefinition, BeanScope, ConfigurableBeanFactory};
use crate::factory::listable_bean_factory::ListableBeanFactory;
use crate::factory::BeanDefinitionRegistry;
use crate::factory::BeanFactory;
use spring_macro::data;
use std::collections::{HashMap, HashSet};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::{Mutex, RwLock};

#[data]
pub struct DefaultListableBeanFactory {
    bean_definition_map: RwLock<HashMap<String, Box<dyn BeanDefinition>>>,
    bean_definition_names: RwLock<Vec<String>>,
    singleton_objects: RwLock<HashMap<String, SharedBean>>,
    currently_in_creation: Mutex<HashSet<String>>,
    post_processor_registry: BeanPostProcessorRegistry,
    environment: Environment,
}

impl BeanDefinitionRegistry for DefaultListableBeanFactory {
    fn contains_bean_definition(&self, bean_name: &str) -> bool {
        self.bean_definition_map
            .read()
            .unwrap()
            .contains_key(bean_name)
    }

    fn get_bean_definition(&self, bean_name: &str) -> Option<&dyn BeanDefinition> {
        let bean_map = self.bean_definition_map.read().unwrap();
        bean_map.get(bean_name).map(|definition| {
            let ptr: *const dyn BeanDefinition = definition.as_ref();
            // SAFETY: callers only use this transiently, matching the previous API contract.
            unsafe { &*ptr }
        })
    }

    fn get_bean_definition_count(&self) -> usize {
        self.bean_definition_map.read().unwrap().len()
    }

    fn get_bean_definition_names(&self) -> Vec<String> {
        self.bean_definition_names.read().unwrap().clone()
    }

    fn is_bean_name_in_use(&self, bean_name: &str) -> bool {
        BeanDefinitionRegistry::contains_bean_definition(self, bean_name)
            || self
                .singleton_objects
                .read()
                .unwrap()
                .contains_key(bean_name)
            || self
                .currently_in_creation
                .lock()
                .unwrap()
                .contains(bean_name)
    }

    fn register_bean_definition(
        &mut self,
        bean_name: &str,
        bean_definition: Box<dyn BeanDefinition>,
    ) {
        self.bean_definition_map
            .write()
            .unwrap()
            .insert(bean_name.to_string(), bean_definition);
        self.bean_definition_names
            .write()
            .unwrap()
            .push(bean_name.to_string());
    }

    fn remove_bean_definition(&mut self, bean_name: &str) {
        self.bean_definition_map.write().unwrap().remove(bean_name);
        self.bean_definition_names
            .write()
            .unwrap()
            .retain(|n| n != bean_name);
    }
}

impl BeanFactory for DefaultListableBeanFactory {
    fn get_bean(&self, name: &str) -> Option<SharedBean> {
        self.singleton_objects.read().unwrap().get(name).cloned()
    }

    fn is_singleton(&self, name: &str) -> bool {
        self.bean_definition_map
            .read()
            .unwrap()
            .get(name)
            .map(|definition| definition.get_scope() == BeanScope::Singleton)
            .unwrap_or_else(|| self.singleton_objects.read().unwrap().contains_key(name))
    }

    fn contains_bean(&self, name: &str) -> bool {
        self.bean_definition_map.read().unwrap().contains_key(name)
            || self.singleton_objects.read().unwrap().contains_key(name)
    }

    fn do_create_bean(&self, name: &str) -> Option<SharedBean> {
        let (dependencies, scope) = {
            let bean_map = self.bean_definition_map.read().unwrap();
            let Some(definition) = bean_map.get(name) else {
                self.currently_in_creation.lock().unwrap().remove(name);
                return None;
            };
            (definition.get_dependencies(), definition.get_scope())
        };

        if scope == BeanScope::Singleton {
            if let Some(bean) = self.singleton_objects.read().unwrap().get(name).cloned() {
                return Some(bean);
            }
        }

        {
            let mut creating = self.currently_in_creation.lock().unwrap();
            if creating.contains(name) {
                eprintln!(
                    "[spring-beans] circular dependency detected at bean '{}'",
                    name
                );
                return None;
            }
            creating.insert(name.to_string());
        }

        for dep in &dependencies {
            if self.currently_in_creation.lock().unwrap().contains(dep) {
                eprintln!(
                    "[spring-beans] circular dependency detected: {} -> {}",
                    name, dep
                );
                self.currently_in_creation.lock().unwrap().remove(name);
                return None;
            }
            self.do_create_bean(dep);
        }

        let deps_snapshot: HashMap<String, SharedBean> = {
            let singletons = self.singleton_objects.read().unwrap();
            dependencies
                .iter()
                .filter_map(|dep_name| {
                    singletons
                        .get(dep_name)
                        .cloned()
                        .map(|b| (dep_name.clone(), b))
                })
                .collect()
        };

        if dependencies
            .iter()
            .any(|dep| !deps_snapshot.contains_key(dep))
        {
            eprintln!(
                "[spring-beans] bean '{}' creation aborted: unresolved dependencies {:?}",
                name, dependencies
            );
            self.currently_in_creation.lock().unwrap().remove(name);
            return None;
        }

        let instance = {
            let bean_map = self.bean_definition_map.read().unwrap();
            let definition = bean_map.get(name)?;
            match catch_unwind(AssertUnwindSafe(|| {
                definition.create_instance(&deps_snapshot, &self.environment.as_map())
            })) {
                Ok(instance) => instance,
                Err(_) => {
                    eprintln!(
                        "[spring-beans] bean '{}' creation aborted due to supplier panic",
                        name
                    );
                    self.currently_in_creation.lock().unwrap().remove(name);
                    return None;
                }
            }
        };

        match scope {
            BeanScope::Singleton => {
                self.singleton_objects
                    .write()
                    .unwrap()
                    .insert(name.to_string(), instance.clone());
                self.currently_in_creation.lock().unwrap().remove(name);
                Some(instance)
            }
            BeanScope::Prototype => {
                self.currently_in_creation.lock().unwrap().remove(name);
                Some(instance)
            }
        }
    }
}

impl ConfigurableBeanFactory for DefaultListableBeanFactory {
    fn register_singleton(&mut self, bean_name: &str, singleton_object: SharedBean) {
        self.singleton_objects
            .write()
            .unwrap()
            .insert(bean_name.to_string(), singleton_object);
    }

    fn destroy_singleton(&mut self, bean_name: &str) {
        self.singleton_objects.write().unwrap().remove(bean_name);
    }

    fn destroy_singletons(&mut self) {
        self.singleton_objects.write().unwrap().clear();
        self.currently_in_creation.lock().unwrap().clear();
    }
}

impl ListableBeanFactory for DefaultListableBeanFactory {
    fn contains_bean_definition(&self, name: &str) -> bool {
        self.bean_definition_map.read().unwrap().contains_key(name)
    }

    fn get_bean_definition_count(&self) -> usize {
        self.bean_definition_map.read().unwrap().len()
    }

    fn get_bean_definition_names(&self) -> Vec<String> {
        self.bean_definition_names.read().unwrap().clone()
    }

    fn get_bean_names_for_type<T>(&self, type_id: std::any::TypeId) -> Vec<String> {
        self.bean_definition_map
            .read()
            .unwrap()
            .iter()
            .filter(|(_, bd)| bd.as_ref().get_type_id() == type_id)
            .map(|(name, _)| name.clone())
            .collect::<Vec<_>>()
    }

    fn get_beans_of_type<T: 'static>(&self) -> Vec<SharedBean> {
        self.singleton_objects
            .read()
            .unwrap()
            .values()
            .filter(|obj| obj.as_ref().downcast_ref::<T>().is_some())
            .cloned()
            .collect()
    }

    fn get_bean_definition_names_for_annotation(&self, annotation: &str) -> Vec<String> {
        self.bean_definition_map
            .read()
            .unwrap()
            .iter()
            .filter(|(_, bd)| bd.as_ref().has_annotation(annotation))
            .map(|(name, _)| name.clone())
            .collect::<Vec<_>>()
    }
}

impl DefaultListableBeanFactory {
    pub fn new() -> Self {
        Self {
            bean_definition_map: RwLock::new(HashMap::new()),
            bean_definition_names: RwLock::new(Vec::new()),
            singleton_objects: RwLock::new(HashMap::new()),
            currently_in_creation: Mutex::new(HashSet::new()),
            post_processor_registry: BeanPostProcessorRegistry::new(),
            environment: Environment::new(),
        }
    }

    pub fn register_post_processor(
        &mut self,
        processor: Box<dyn crate::bean::bean_post_processor::BeanPostProcessor + Send + Sync>,
    ) {
        self.post_processor_registry.register(processor);
    }
}
impl Default for DefaultListableBeanFactory {
    fn default() -> Self {
        Self::new()
    }
}
