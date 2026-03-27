use crate::aspect::advice::{Advice, AdviceKind, JoinPoint};
use crate::aspect::advisor::Advisor;
use crate::aspect::pointcut::Pointcut;
use std::sync::MutexGuard;
use std::sync::{Mutex, OnceLock};

/// Global registry of all `Advisor`s collected from `#[Aspect]` classes.
///
/// `spring-macro` submits `AspectRegistration` entries at link time via
/// `inventory`.  `AopProxyRegistry::initialize()` is called once by
/// `Application::run()` to convert those entries into `Advisor`s stored here.
static REGISTRY: OnceLock<Mutex<Vec<Advisor>>> = OnceLock::new();

fn registry() -> &'static Mutex<Vec<Advisor>> {
    REGISTRY.get_or_init(|| Mutex::new(Vec::new()))
}

fn registry_guard() -> MutexGuard<'static, Vec<Advisor>> {
    match registry().lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

pub struct AopProxyRegistry;

impl AopProxyRegistry {
    /// Register an advisor programmatically (used by `Application::run()` after
    /// converting `AspectRegistration` inventory entries).
    pub fn register(advisor: Advisor) {
        registry_guard().push(advisor);
    }

    /// Convenience: register a `Before` advice for `"beanName::methodName"`.
    pub fn register_before(expr: &str, f: impl Fn(&JoinPoint) + Send + Sync + 'static) {
        if let Ok(pc) = Pointcut::parse(expr) {
            Self::register(Advisor::new(pc, Advice::before(f)));
        } else {
            eprintln!("[spring-aop] skip invalid before pointcut: {}", expr);
        }
    }

    /// Convenience: register an `After` advice for `"beanName::methodName"`.
    pub fn register_after(expr: &str, f: impl Fn(&JoinPoint) + Send + Sync + 'static) {
        if let Ok(pc) = Pointcut::parse(expr) {
            Self::register(Advisor::new(pc, Advice::after(f)));
        } else {
            eprintln!("[spring-aop] skip invalid after pointcut: {}", expr);
        }
    }

    /// Convenience: register an `Around` advice for `"beanName::methodName"`.
    pub fn register_around(expr: &str, f: impl Fn(&JoinPoint) + Send + Sync + 'static) {
        if let Ok(pc) = Pointcut::parse(expr) {
            Self::register(Advisor::new(pc, Advice::around(f)));
        } else {
            eprintln!("[spring-aop] skip invalid around pointcut: {}", expr);
        }
    }

    /// Call all `Before` (and `Around` pre-) advices that match
    /// `(bean_name, method_name)`.
    pub fn fire_before(bean_name: &str, method_name: &str) {
        let jp = JoinPoint::new(bean_name, method_name);
        let advisors = registry_guard();
        for advisor in advisors.iter() {
            if advisor.pointcut.matches(bean_name, method_name) {
                match advisor.advice.kind {
                    AdviceKind::Before | AdviceKind::Around => {
                        (advisor.advice.handler)(&jp);
                    }
                    _ => {}
                }
            }
        }
    }

    /// Call all `After` (and `Around` post-) advices that match
    /// `(bean_name, method_name)`.
    pub fn fire_after(bean_name: &str, method_name: &str) {
        let jp = JoinPoint::new(bean_name, method_name);
        let advisors = registry_guard();
        for advisor in advisors.iter() {
            if advisor.pointcut.matches(bean_name, method_name) {
                match advisor.advice.kind {
                    AdviceKind::After | AdviceKind::Around => {
                        (advisor.advice.handler)(&jp);
                    }
                    _ => {}
                }
            }
        }
    }

    /// Returns `true` if any advisor targets the given bean.
    pub fn has_advisors_for(bean_name: &str) -> bool {
        registry_guard()
            .iter()
            .any(|a| a.pointcut.bean_name == bean_name)
    }
}

// ── AopGuard ─────────────────────────────────────────────────────────────────

/// RAII guard: calls `fire_after` automatically when it goes out of scope.
///
/// Used by the `#[AopMethods]` proc-macro so that `fire_after` is triggered
/// whether the wrapped method returns normally **or** via an early `return`.
///
/// ```rust,ignore
/// pub fn place_order(&self, item: &str) {
///     AopProxyRegistry::fire_before("orderService", "place_order");
///     let _guard = AopGuard::new("orderService", "place_order");
///     // original body …
/// }  // ← _guard drops here → fire_after called automatically
/// ```
pub struct AopGuard {
    bean_name: &'static str,
    method_name: &'static str,
}

impl AopGuard {
    pub fn new(bean_name: &'static str, method_name: &'static str) -> Self {
        AopGuard {
            bean_name,
            method_name,
        }
    }
}

impl Drop for AopGuard {
    fn drop(&mut self) {
        AopProxyRegistry::fire_after(self.bean_name, self.method_name);
    }
}
