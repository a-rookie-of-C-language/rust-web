use super::application_context::ApplicationContext;
use super::lifecycle::Lifecycle;

pub trait ConfigurableApplicationContext: ApplicationContext + Lifecycle {
    fn refresh(&mut self);
    fn close(&mut self);
    fn is_active(&self) -> bool;
}
