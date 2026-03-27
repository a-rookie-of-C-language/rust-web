pub mod application;

pub use application::Application;

// Re-export all proc-macros so users only need `spring-boot` as a dependency.
pub use spring_macro::{
    After, AopMethods, Around, Aspect, Bean, Before, Component, ConditionalOnProperty, Lazy, Scope,
    Value,
};

// Re-export AOP interceptor so users can call AopProxyRegistry::fire_before / fire_after
pub use spring_aop::{AdviceKind, AopGuard, AopProxyRegistry, AspectRegistration, JoinPoint};

// Re-export the ApplicationContext trait so users can call get_bean / do_create_bean
// without importing spring_context directly.
pub use spring_context::context::application_context::ApplicationContext;

// Re-export SpEL evaluator so proc-macro generated code can use spring_boot::spel::eval
// without requiring users to add spring-expression as a direct dependency.
pub mod spel {
    pub use spring_expression::eval;
}

// Re-export spring-data types & Repository trait so proc-macro generated code and
// users can use spring_boot::data::* without adding spring-data as a direct dep.
pub mod data {
    pub use spring_data::{InMemoryRepository, Repository};
}

// Re-export #[Repository] proc-macro alongside other macros.
pub use spring_macro::Repository;

// Re-export spring-web types so proc-macro generated code can reference
// spring_boot::web::* and users only need spring-boot as a dependency.
pub mod web {
    pub use spring_web::{
        BeanHandlerFn, Handler, HttpMethod, HttpRequest, HttpResponse, HttpServer, PlainHandlerFn,
        RouteRegistration, Router, StatusCode,
    };
}

// Re-export web macros and HttpServer at top level for ergonomic use.
pub use spring_macro::{
    DeleteMapping, GetMapping, PatchMapping, PostMapping, PutMapping, RestController,
};
pub use spring_web::HttpServer;
