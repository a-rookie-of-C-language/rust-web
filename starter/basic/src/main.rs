use spring_boot::{Application, ApplicationContext, Component, Value};

#[Component]
#[derive(Debug, Default, Clone)]
struct StarterConfig {
    #[Value("${app.name:starter}")]
    app_name: String,
    #[Value("${server.port:8080}")]
    server_port: u16,
}

fn main() {
    let context = Application::run();
    if let Some(bean) = context.get_bean("starterConfig") {
        if let Some(cfg) = bean.downcast_ref::<StarterConfig>() {
            println!("app={} port={}", cfg.app_name, cfg.server_port);
        }
    }
}
