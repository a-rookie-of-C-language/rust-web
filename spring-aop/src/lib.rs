pub mod aspect;
pub mod framework;
pub mod proxy;

pub use aspect::advice::{Advice, AdviceKind, JoinPoint};
pub use aspect::advisor::Advisor;
pub use aspect::pointcut::Pointcut;
pub use framework::aop_config::AopConfig;
pub use proxy::aop_proxy::{AopGuard, AopProxyRegistry};

// ── Inventory-based aspect registration ────────────────────────────────────

/// One entry submitted (at link time, via `inventory::submit!`) by each
/// `#[Before]` / `#[After]` / `#[Around]` macro invocation.
///
/// `Application::run()` iterates all collected entries and registers them with
/// `AopProxyRegistry`.
pub struct AspectRegistration {
    /// Pointcut expression, e.g. `"userService::save"`.
    pub pointcut: &'static str,
    /// Whether this is a Before, After, or Around advice.
    pub kind: AdviceKind,
    /// The advice function.
    pub handler: fn(&JoinPoint),
}

inventory::collect!(AspectRegistration);

/// Called once by `Application::run()` to populate `AopProxyRegistry` from
/// all statically-submitted `AspectRegistration` entries.
pub fn initialize_aop() {
    for reg in inventory::iter::<AspectRegistration>() {
        let Ok(pc) = Pointcut::parse(reg.pointcut) else {
            eprintln!("[spring-aop] skip invalid pointcut: {}", reg.pointcut);
            continue;
        };
        let kind = reg.kind;
        let handler = reg.handler;
        let advice = Advice {
            kind,
            handler: Box::new(move |jp: &JoinPoint| handler(jp)),
        };
        AopProxyRegistry::register(Advisor::new(pc, advice));
    }
}
