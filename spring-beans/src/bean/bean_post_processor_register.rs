use crate::bean::bean_post_processor::BeanPostProcessor;

pub struct BeanPostProcessorRegistry {
    processors: Vec<Box<dyn BeanPostProcessor>>,
}

impl BeanPostProcessorRegistry {
    pub fn new() -> Self {
        Self {
            processors: Vec::new(),
        }
    }

    pub fn register(&mut self, processor: Box<dyn BeanPostProcessor>) {
        self.processors.push(processor);
        self.processors.sort_by_key(|p| p.order());
    }

    pub fn get_processors(&self) -> &Vec<Box<dyn BeanPostProcessor>> {
        &self.processors
    }

    pub fn apply_before_initialization(&self, bean_name: &str, bean: &mut dyn std::any::Any) {
        for processor in &self.processors {
            processor.post_process_before_initialization(bean_name, bean);
        }
    }
    pub fn apply_after_initialization(&self, bean_name: &str, bean: &mut dyn std::any::Any) {
        for processor in &self.processors {
            processor.post_process_after_initialization(bean_name, bean);
        }
    }

    pub fn len(&self) -> usize {
        self.processors.len()
    }

    pub fn is_empty(&self) -> bool {
        self.processors.is_empty()
    }
}

impl Default for BeanPostProcessorRegistry {
    fn default() -> Self {
        Self::new()
    }
}
