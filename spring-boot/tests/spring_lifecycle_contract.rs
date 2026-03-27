use spring_boot::{Application, ApplicationContext, Component};
use spring_context::context::configurable_application_context::ConfigurableApplicationContext;

#[Component]
#[derive(Debug, Default, Clone)]
struct LifecycleContractBean;

#[test]
fn close_clears_singleton_cache() {
    let mut context = Application::run();
    assert!(context.get_bean("lifecycleContractBean").is_some());
    context.close();
    assert!(context.get_bean("lifecycleContractBean").is_none());
}
