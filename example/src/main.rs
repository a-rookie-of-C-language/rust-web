use spring_boot::{
    After, AopMethods, Application, ApplicationContext, Around, Aspect, Bean, Before, Component,
    JoinPoint, Repository,
};

// ── 基础 bean ──────────────────────────────────────────────────────────────────

#[Component]
#[derive(Debug, Default, Clone)]
struct Person {
    id: i32,
    name: String,
}

// ── #[autowired] 依赖注入 ──────────────────────────────────────────────────────

#[Component]
#[derive(Debug, Default, Clone)]
struct User {
    #[autowired]
    person: Person,
    id: i32,
    name: String,
}

// ── #[Scope("prototype")] ─────────────────────────────────────────────────────
// 每次 do_create_bean 都创建新实例，不缓存到 singleton_objects

#[Component]
#[spring_boot::Scope("prototype")]
#[derive(Debug, Default, Clone)]
struct RequestContext {
    request_id: i32,
}

// ── #[Lazy] ───────────────────────────────────────────────────────────────────
// refresh() 时不主动创建，第一次 get_bean() 时才初始化

#[Component]
#[spring_boot::Lazy]
#[derive(Debug, Default, Clone)]
struct HeavyService {
    initialized: bool,
}

// ── #[Bean] ───────────────────────────────────────────────────────────────────
// 函数式定义 bean，类似 Java @Configuration + @Bean

#[derive(Debug, Clone)]
struct AppConfig {
    version: String,
    max_connections: u32,
}

#[Bean(name = "appConfig")]
fn create_app_config() -> AppConfig {
    AppConfig {
        version: "1.0.0".to_string(),
        max_connections: 100,
    }
}

// ── #[Value] 配置注入 ────────────────────────────────────────────────────────────────────
// 字段从 application.properties 注入，相当于 Java @Value

#[Component]
#[derive(Debug, Default, Clone)]
struct ServerConfig {
    #[Value("${server.port:8080}")]
    port: i32,
    #[Value("${app.name:rust-spring}")]
    app_name: String,
    #[Value("${app.version:1.0.0}")]
    version: String,
    #[Value("${app.max-connections:100}")]
    max_connections: u32,
}

// ── SpEL 表达式注入 ───────────────────────────────────────────────────────────────────────
// 字段值由 #{表达式} SpEL 计算后注入，支持算术、比较、三元、字符串方法等

#[Component]
#[derive(Debug, Default, Clone)]
struct SpelConfig {
    // 三元表达式：端口大于 8000 时取双倍，否则取原值
    #[Value("#{${server.port:8080} > 8000 ? ${server.port:8080} * 2 : ${server.port:8080}}")]
    double_port: i32,
    // 字符串方法：应用名转大写
    #[Value("#{${app.name:rust-spring}.toUpperCase()}")]
    app_name_upper: String,
    // 算术：最大连接数 + 50
    #[Value("#{${app.max-connections:100} + 50}")]
    extra_connections: u32,
    // 比较：版本等于 2.0.0？
    #[Value("#{${app.version:1.0.0} == '2.0.0'}")]
    is_v2: bool,
}

// ── AOP 切面演示 ────────────────────────────────────────────────────────────────
//
// OrderService: 被拦截的 bean

#[Component]
#[derive(Debug, Default, Clone)]
struct OrderService {
    order_count: u32,
}

#[AopMethods] // 透明织入：所有 pub fn 自动被 Before/After/Around 拦截，无需手动调用 fire_*
impl OrderService {
    /// 下单方法 —— 由 LogAspect 进行 Before / After 拦截。
    pub fn place_order(&self, item: &str) {
        println!(
            "[OrderService] placing order for: {}, count={}",
            item, self.order_count
        );
    }
}

// LogAspect: 切面类（用于标识切面——提示性）
#[Aspect]
struct LogAspect;

// ── #[ConditionalOnProperty] 演示 ───────────────────────────────────────────
// CacheService: feature.cache.enabled=true 时才注册（properties 里已设置）
#[Component]
#[spring_boot::ConditionalOnProperty("feature.cache.enabled", having = "true")]
#[derive(Debug, Default, Clone)]
struct CacheService {
    #[Value("${cache.ttl:300}")]
    ttl: u32,
}

// AnalyticsService: feature.analytics.enabled 未设置，因此不会注册
#[Component]
#[spring_boot::ConditionalOnProperty("feature.analytics.enabled", having = "true")]
#[derive(Debug, Default, Clone)]
struct AnalyticsService;
// ── Spring Data 风格 Repository 演示 ────────────────────────────────
// Product: 普通 Rust 结构体，即实体类型
#[derive(Debug, Clone)]
struct Product {
    name: String,
    price: f64,
    stock: u32,
}

impl Product {
    fn new(name: &str, price: f64, stock: u32) -> Self {
        Self {
            name: name.to_string(),
            price,
            stock,
        }
    }
}

// ProductRepository: #[Repository(Product)] 宏自动生成内存 CRUD + IoC 注册
#[Repository(Product)]
struct ProductRepository;
// 切面函数必须是模块级别的独立函数（非 impl 方法）
#[Before("orderService::place_order")]
fn log_before(jp: &JoinPoint) {
    println!(
        "[AOP][Before]  {}.{}() is about to execute",
        jp.bean_name, jp.method_name
    );
}

#[After("orderService::place_order")]
fn log_after(jp: &JoinPoint) {
    println!(
        "[AOP][After]   {}.{}() has finished",
        jp.bean_name, jp.method_name
    );
}

#[Around("orderService::place_order")]
fn log_around(jp: &JoinPoint) {
    // Around fires on both sides (before via fire_before, after via fire_after).
    println!(
        "[AOP][Around]  intercepting {}.{}()",
        jp.bean_name, jp.method_name
    );
}
// ── main ──────────────────────────────────────────────────────────────────────

fn main() {
    let context = Application::run();

    // 1. 普通 singleton bean
    if let Some(bean) = context.get_bean("person") {
        if let Some(person) = bean.as_ref().downcast_ref::<Person>() {
            println!(
                "[Singleton]  person bean: id={}, name='{}'",
                person.id, person.name
            );
        }
    }

    // 2. autowired 注入
    if let Some(bean) = context.get_bean("user") {
        if let Some(user) = bean.as_ref().downcast_ref::<User>() {
            println!(
                "[Autowired]  user bean:   id={}, name='{}', person='{}'",
                user.id, user.name, user.person.name
            );
        }
    }

    // 3. Prototype bean — 每次 do_create_bean 产生新实例
    context.do_create_bean("requestContext");
    let request_ctx_probe = RequestContext::default();
    println!("[Prototype]  requestContext: prototype bean (not cached in singleton store)");
    println!(
        "[Prototype]  probe request_id={}",
        request_ctx_probe.request_id
    );

    // 4. Lazy singleton — refresh() 时跳过，首次 get_bean 时触发创建
    if context.get_bean("heavyService").is_none() {
        println!(
            "[Lazy]       heavyService: not yet initialized (lazy=true, needs do_create_bean)"
        );
        context.do_create_bean("heavyService");
    }
    if let Some(bean) = context.get_bean("heavyService") {
        if let Some(svc) = bean.as_ref().downcast_ref::<HeavyService>() {
            println!(
                "[Lazy]       heavyService initialized: initialized={}",
                svc.initialized
            );
        }
    }

    // 5. @Bean 函数式定义
    if let Some(bean) = context.get_bean("appConfig") {
        if let Some(cfg) = bean.as_ref().downcast_ref::<AppConfig>() {
            println!(
                "[Bean]       appConfig: version={}, max_connections={}",
                cfg.version, cfg.max_connections
            );
        }
    }

    // 6. #[Value] 配置注入
    if let Some(bean) = context.get_bean("serverConfig") {
        if let Some(cfg) = bean.as_ref().downcast_ref::<ServerConfig>() {
            println!("[Value]      serverConfig: {:?}", cfg);
        }
    }

    // 6b. #[Value("#{...}")] SpEL 表达式注入
    if let Some(bean) = context.get_bean("spelConfig") {
        if let Some(cfg) = bean.as_ref().downcast_ref::<SpelConfig>() {
            println!(
                "[SpEL]       double_port (port*2 if >8000): {}",
                cfg.double_port
            );
            println!(
                "[SpEL]       app_name_upper (toUpperCase): {}",
                cfg.app_name_upper
            );
            println!(
                "[SpEL]       extra_connections (max+50):    {}",
                cfg.extra_connections
            );
            println!("[SpEL]       is_v2 (version=='2.0.0'):       {}", cfg.is_v2);
        }
    }

    // 7. AOP 切面拦截演示
    let _aspect_marker = LogAspect;
    if let Some(bean) = context.get_bean("orderService") {
        if let Some(svc) = bean.as_ref().downcast_ref::<OrderService>() {
            println!("\n[AOP] Calling orderService.place_order(\"laptop\")...");
            svc.place_order("laptop");
        }
    }

    // 8. #[ConditionalOnProperty] 演示
    println!("\n[Conditional]");
    // CacheService: feature.cache.enabled=true -> 应该被注册
    match context.get_bean("cacheService") {
        Some(bean) if bean.as_ref().downcast_ref::<CacheService>().is_some() => {
            let svc = bean.as_ref().downcast_ref::<CacheService>().unwrap();
            println!(
                "  cacheService registered   (feature.cache.enabled=true):  ttl={}s",
                svc.ttl
            );
        }
        _ => println!("  cacheService NOT registered"),
    }
    // AnalyticsService: feature.analytics.enabled 未设置 -> 不应该被注册
    match context.get_bean("analyticsService") {
        None => println!("  analyticsService NOT registered (feature.analytics.enabled not set) ✓"),
        Some(_) => println!("  analyticsService registered (unexpected)"),
    }
    // 9. Spring Data 风格 Repository CRUD 演示
    println!("\n[Repository]");
    if let Some(bean) = context.get_bean("productRepository") {
        if let Some(repo) = bean.downcast_ref::<ProductRepository>() {
            // save
            let id1 = repo.save(Product::new("Rust Book", 39.9, 100));
            let id2 = repo.save(Product::new("Cargo Mug", 9.9, 50));
            let id3 = repo.save(Product::new("Ferris Plush", 19.9, 200));
            println!("  saved 3 products, ids: {}, {}, {}", id1, id2, id3);
            println!("  count: {}", repo.count());

            // find_by_id
            repo.find_by_id(id1, |p| {
                if let Some(p) = p {
                    println!("  find id={}: {:?}", id1, p);
                }
            });

            // update
            let updated = repo.update(id2, Product::new("Cargo Mug XL", 14.9, 30));
            println!("  update id={}: {}", id2, updated);

            // find_all_cloned
            let all = repo.find_all_cloned();
            println!("  find_all ({} items):", all.len());
            for (id, p) in &all {
                println!(
                    "    id={} name='{}' price={:.1} stock={}",
                    id, p.name, p.price, p.stock
                );
            }

            // delete
            let deleted = repo.delete_by_id(id3);
            println!(
                "  delete id={}: {}, count now: {}",
                id3,
                deleted,
                repo.count()
            );

            // exists
            println!("  exists id={}: {}", id3, repo.exists_by_id(id3));
        }
    }
}
