use spring_boot::{Application, ApplicationContext, Component};

#[Component]
#[derive(Debug, Default, Clone)]
struct ProfileConfig {
    #[spring_boot::Value("${app.name:default-app}")]
    app_name: String,
    #[spring_boot::Value("${server.port:8080}")]
    port: u16,
}

fn main() {
    let context = Application::run();
    if let Some(bean) = context.get_bean("profileConfig") {
        if let Some(cfg) = bean.downcast_ref::<ProfileConfig>() {
            println!("profile-demo app={} port={}", cfg.app_name, cfg.port);
        }
    }
}
