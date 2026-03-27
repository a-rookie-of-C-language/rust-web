use spring_aop::initialize_aop;
use spring_beans::bean::bean_post_processor::DefaultBeanPostProcessor;
use spring_beans::env::{Environment, MapPropertySource, PropertiesLoader};
use spring_beans::factory::BeanDefinitionRegistry;
use spring_context::context::support::AbstractApplicationContext;
use spring_context::context::ConfigurableApplicationContext;

/// Spring Boot 应用入口，对标 Java 的 SpringApplication。
pub struct Application;

impl Application {
    /// 自动扫描所有 #[Component] bean，注册到容器，refresh 后返回。
    /// 对标 Java 的 SpringApplication.run()。
    pub fn run() -> AbstractApplicationContext {
        let mut context = AbstractApplicationContext::default();

        // 先加载环境，供条件过滤使用
        let mut environment = Environment::new();
        load_environment_layers(&mut environment);

        // 遍历所有通过 inventory::submit! 注册的 BeanRegistration
        // 按条件过滤后再注册
        for registration in inventory::iter::<spring_beans::registry::BeanRegistration> {
            let definition = (registration.definition)();

            // 检查 #[ConditionalOnProperty] 条件
            if let Some((key, expected)) = definition.get_condition() {
                let actual = environment.get_property(key).unwrap_or("");
                if actual != expected {
                    continue; // 条件不满足，跳过该 bean
                }
            }

            let name = definition.get_name().to_string();
            context.register_bean_definition(&name, Box::new(definition));
        }

        context.set_environment(environment);

        // 注册默认的 BeanPostProcessor
        context.register_post_processor(Box::new(DefaultBeanPostProcessor {}));

        // 初始化 AOP：将所有 inventory 提交的 AspectRegistration 转为 Advisor
        initialize_aop();

        context.refresh();
        context
    }
}

fn load_environment_layers(environment: &mut Environment) {
    if let Ok(props) = PropertiesLoader::load("application.properties") {
        let source = MapPropertySource::new("application.properties", props);
        environment.merge_from_override(&source);
    }

    if let Ok(profile) = std::env::var("SPRING_PROFILE") {
        let profile = profile.trim();
        if !profile.is_empty() {
            let profile_path = format!("application-{}.properties", profile);
            if let Ok(props) = PropertiesLoader::load(&profile_path) {
                let source = MapPropertySource::new(&profile_path, props);
                environment.merge_from_override(&source);
            }
        }
    }

    for (key, value) in std::env::vars() {
        if let Some(prop_key) = key.strip_prefix("SPRING_PROP_") {
            let normalized_key = prop_key.to_ascii_lowercase().replace('_', ".");
            environment.set_property(normalized_key, value);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::load_environment_layers;
    use spring_beans::env::Environment;
    use std::sync::{Mutex, OnceLock};

    fn test_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    #[test]
    fn profile_and_env_override_precedence() {
        let _guard = test_lock().lock().expect("lock should be available");

        let original_dir = std::env::current_dir().expect("cwd should be readable");
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time should be valid")
            .as_nanos();
        let tmp_dir = std::env::temp_dir().join(format!(
            "rust_spring_env_test_{}_{}",
            std::process::id(),
            unique
        ));
        let _ = std::fs::remove_dir_all(&tmp_dir);
        std::fs::create_dir_all(&tmp_dir).expect("tmp dir should be created");

        std::fs::write(
            tmp_dir.join("application.properties"),
            "app.name=base\nserver.port=8080\n",
        )
        .expect("base props should be written");
        std::fs::write(
            tmp_dir.join("application-test.properties"),
            "app.name=profile\n",
        )
        .expect("profile props should be written");

        std::env::set_current_dir(&tmp_dir).expect("cwd switch should work");
        std::env::set_var("SPRING_PROFILE", "test");
        std::env::set_var("SPRING_PROP_SERVER_PORT", "9090");

        let mut env = Environment::new();
        load_environment_layers(&mut env);

        assert_eq!(env.get_property("app.name"), Some("profile"));
        assert_eq!(env.get_property("server.port"), Some("9090"));

        std::env::remove_var("SPRING_PROFILE");
        std::env::remove_var("SPRING_PROP_SERVER_PORT");
        std::env::set_current_dir(original_dir).expect("cwd restore should work");
        let _ = std::fs::remove_dir_all(&tmp_dir);
    }
}
