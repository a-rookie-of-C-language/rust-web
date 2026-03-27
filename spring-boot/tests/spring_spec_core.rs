use std::sync::atomic::{AtomicUsize, Ordering};

use spring_boot::{
    After, AopMethods, Application, ApplicationContext, Around, Aspect, Bean, Before, Component,
    JoinPoint, Repository,
};

static SPEC_EAGER_INITS: AtomicUsize = AtomicUsize::new(0);
static SPEC_LAZY_INITS: AtomicUsize = AtomicUsize::new(0);
static SPEC_BEFORE_COUNT: AtomicUsize = AtomicUsize::new(0);
static SPEC_AFTER_COUNT: AtomicUsize = AtomicUsize::new(0);
static SPEC_AROUND_COUNT: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug, Clone)]
struct SpecProduct {
    name: String,
    price: f64,
}

impl SpecProduct {
    fn new(name: &str, price: f64) -> Self {
        Self {
            name: name.to_string(),
            price,
        }
    }
}

#[Component]
#[derive(Debug, Clone)]
struct SpecEagerProbe {
    id: usize,
}

impl Default for SpecEagerProbe {
    fn default() -> Self {
        let id = SPEC_EAGER_INITS.fetch_add(1, Ordering::SeqCst) + 1;
        Self { id }
    }
}

#[Component]
#[spring_boot::Lazy]
#[derive(Debug, Clone)]
struct SpecLazyProbe {
    id: usize,
}

impl Default for SpecLazyProbe {
    fn default() -> Self {
        let id = SPEC_LAZY_INITS.fetch_add(1, Ordering::SeqCst) + 1;
        Self { id }
    }
}

#[Component]
#[spring_boot::Scope("prototype")]
#[derive(Debug, Default, Clone)]
struct SpecPrototypeProbe;

#[Component]
#[derive(Debug, Default, Clone)]
struct SpecGreeter {
    greeting: String,
}

#[Component]
#[derive(Debug, Default, Clone)]
struct SpecUserService {
    #[autowired]
    greeter: SpecGreeter,
    #[Value("${spec.http.port:8080}")]
    port: u16,
    #[Value("#{40 + 2}")]
    answer: i32,
}

#[Component]
#[spring_boot::ConditionalOnProperty("feature.cache.enabled", having = "true")]
#[derive(Debug, Default, Clone)]
struct SpecConditionalCache;

#[Component]
#[spring_boot::ConditionalOnProperty("feature.analytics.enabled", having = "true")]
#[derive(Debug, Default, Clone)]
struct SpecConditionalAnalytics;

#[Bean(name = "specAppName")]
fn create_spec_app_name() -> String {
    "rust-spring-spec".to_string()
}

#[Repository(SpecProduct)]
struct SpecProductRepository;

#[Component]
#[derive(Debug, Default, Clone)]
struct SpecOrderService;

#[AopMethods]
impl SpecOrderService {
    pub fn place_order(&self, item: &str) -> String {
        format!("ordered:{}", item)
    }
}

#[Aspect]
struct SpecLogAspect;

#[Before("specOrderService::place_order")]
fn spec_before(_jp: &JoinPoint) {
    SPEC_BEFORE_COUNT.fetch_add(1, Ordering::SeqCst);
}

#[After("specOrderService::place_order")]
fn spec_after(_jp: &JoinPoint) {
    SPEC_AFTER_COUNT.fetch_add(1, Ordering::SeqCst);
}

#[Around("specOrderService::place_order")]
fn spec_around(_jp: &JoinPoint) {
    SPEC_AROUND_COUNT.fetch_add(1, Ordering::SeqCst);
}

#[test]
fn spring_core_spec_contract() {
    SPEC_EAGER_INITS.store(0, Ordering::SeqCst);
    SPEC_LAZY_INITS.store(0, Ordering::SeqCst);
    SPEC_BEFORE_COUNT.store(0, Ordering::SeqCst);
    SPEC_AFTER_COUNT.store(0, Ordering::SeqCst);
    SPEC_AROUND_COUNT.store(0, Ordering::SeqCst);

    let _aspect_marker = SpecLogAspect;
    let mut context = Application::run();

    assert!(context.contains_bean("specEagerProbe"));
    assert_eq!(SPEC_EAGER_INITS.load(Ordering::SeqCst), 1);

    assert!(context.contains_bean("specLazyProbe"));
    assert!(context.get_bean("specLazyProbe").is_none());
    assert_eq!(SPEC_LAZY_INITS.load(Ordering::SeqCst), 0);
    context.do_create_bean("specLazyProbe");
    assert!(context.get_bean("specLazyProbe").is_some());
    assert_eq!(SPEC_LAZY_INITS.load(Ordering::SeqCst), 1);

    assert!(context.contains_bean("specPrototypeProbe"));
    assert!(!context.is_singleton("specPrototypeProbe"));

    let user = context
        .get_bean("specUserService")
        .and_then(|b| b.downcast_ref::<SpecUserService>())
        .expect("specUserService should exist and type-match");
    assert_eq!(user.port, 8080);
    assert_eq!(user.answer, 42);
    assert_eq!(user.greeter.greeting, String::default());

    assert!(context.get_bean("specConditionalCache").is_none());
    assert!(context.get_bean("specConditionalAnalytics").is_none());

    let app_name = context
        .get_bean("specAppName")
        .and_then(|b| b.downcast_ref::<String>())
        .expect("specAppName bean should be available");
    assert_eq!(app_name, "rust-spring-spec");

    let repo = context
        .get_bean("specProductRepository")
        .and_then(|b| b.downcast_ref::<SpecProductRepository>())
        .expect("specProductRepository should exist");
    let id1 = repo.save(SpecProduct::new("book", 39.9));
    let id2 = repo.save(SpecProduct::new("pen", 2.5));
    assert_eq!(repo.count(), 2);
    assert!(repo.exists_by_id(id1));
    assert!(repo.update(id2, SpecProduct::new("pen-pro", 3.0)));
    repo.find_by_id(id2, |p| {
        let p = p.expect("updated record must exist");
        assert_eq!(p.name, "pen-pro");
        assert_eq!(p.price, 3.0);
    });
    assert!(repo.delete_by_id(id1));
    assert_eq!(repo.count(), 1);

    let order_service = context
        .get_bean("specOrderService")
        .and_then(|b| b.downcast_ref::<SpecOrderService>())
        .expect("specOrderService should exist");
    let out = order_service.place_order("laptop");
    assert_eq!(out, "ordered:laptop");
    assert!(SPEC_BEFORE_COUNT.load(Ordering::SeqCst) >= 1);
    assert!(SPEC_AFTER_COUNT.load(Ordering::SeqCst) >= 1);
    assert!(SPEC_AROUND_COUNT.load(Ordering::SeqCst) >= 2);

    let eager = context
        .get_bean("specEagerProbe")
        .and_then(|b| b.downcast_ref::<SpecEagerProbe>())
        .expect("specEagerProbe should be materialized");
    assert_eq!(eager.id, 1);

    let lazy = context
        .get_bean("specLazyProbe")
        .and_then(|b| b.downcast_ref::<SpecLazyProbe>())
        .expect("specLazyProbe should be materialized after do_create_bean");
    assert_eq!(lazy.id, 1);
}
