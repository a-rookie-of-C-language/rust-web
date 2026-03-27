# rust-spring

> A Rust re-implementation of the Spring Framework core — annotation-driven IoC container, dependency injection, and Spring Boot-style auto-configuration. No XML. Just annotations.
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-2021_edition-orange.svg)](https://www.rust-lang.org)

English | [中文](README.zh.md)

---

## Overview

`rust-spring` ports the essential ideas of the Java Spring ecosystem into idiomatic Rust:

| Java Spring | rust-spring |
|---|---|
| `@Component` | `#[Component]` |
| `@Autowired` | `#[autowired]` (field attribute) |
| `@Bean` | `#[Bean]` (on a function) |
| `@Scope("prototype")` | `#[Scope("prototype")]` |
| `@Lazy` | `#[Lazy]` |
| `@Value("${key:default}")` | `#[Value("${key:default}")]` |
| `SpringApplication.run()` | `Application::run()` |
| `application.properties` | `application.properties` + `application-{profile}.properties` + `SPRING_PROP_*` |

---

## Crate Layout

```
rust-spring/
├── spring-core        # Fundamental utilities and traits
├── spring-beans       # BeanFactory, BeanDefinition, Environment
├── spring-context     # ApplicationContext, bean lifecycle
├── spring-boot        # Application entry point + re-exports (start here)
├── spring-macro       # Proc-macros: #[Component], #[Bean], #[Value], ...
├── spring-aop         # AOP stub (in progress)
├── spring-expression  # SpEL-style expression engine (in progress)
├── spring-util        # Shared utilities
├── example            # Runnable demo of all features
└── initializer        # CLI tool — generates a new rust-spring project
```

Users only need **`spring-boot`** as a dependency. Everything else is pulled in transitively.

---

## Quick Start

### Add the dependency

```toml
# Cargo.toml
[dependencies]
spring-boot = { git = "https://github.com/arookieofc/rust-spring" }
```

### Write your application

```rust
use spring_boot::{Application, ApplicationContext, Component, Value};

#[Component]
#[derive(Debug, Default, Clone)]
struct HelloService {
    #[Value("${greeting:Hello, World}")]
    greeting: String,
}

fn main() {
    let context = Application::run();

    if let Some(bean) = context.get_bean("helloService") {
        if let Some(svc) = bean.downcast_ref::<HelloService>() {
            println!("{}", svc.greeting);
        }
    }
}
```

### Add `application.properties` next to your binary

```properties
greeting=Hello from rust-spring!
```

### Run

```
cargo run
```

---

## Generate a New Project

Use the `initializer` CLI to scaffold a ready-to-run project:

```bash
# From inside the rust-spring workspace
cargo run -p initializer -- --name my-app --output /tmp
cd /tmp/my-app
cargo run
```

Generated structure:

```
my-app/
├── Cargo.toml              # spring-boot git dependency, nothing else
├── application.properties  # sample config values
└── src/
    └── main.rs             # HelloService + AppConfig demo
```

---

## Annotation Reference

### `#[Component]`

Marks a struct as a managed bean. rust-spring registers it automatically at startup.

```rust
#[Component]
#[derive(Debug, Default, Clone)]
struct UserService { ... }
```

Bean name defaults to the camelCase struct name (`UserService` → `"userService"`).

---

### `#[autowired]` (field)

Injects another bean into a field. The field type must itself be a `#[Component]`.

```rust
#[Component]
#[derive(Debug, Default, Clone)]
struct OrderService {
    #[autowired]
    user_service: UserService,
}
```

---

### `#[Bean]`

Defines a bean via a factory function, equivalent to `@Configuration + @Bean` in Java.

```rust
#[Bean(name = "dataSource")]
fn create_data_source() -> DataSource {
    DataSource { url: "postgres://localhost/mydb".into() }
}
```

---

### `#[Value("${key:default}")]` (field)

Injects a value from `application.properties`. Supports a default after `:`.

```rust
#[Component]
#[derive(Debug, Default, Clone)]
struct Config {
    #[Value("${server.port:8080}")]
    port: i32,

    #[Value("${app.name:my-app}")]
    name: String,
}
```

---

### `#[Scope("prototype")]`

Creates a new instance on every explicit `do_create_bean` call instead of reusing the singleton.

```rust
#[Component]
#[Scope("prototype")]
#[derive(Debug, Default, Clone)]
struct RequestContext { ... }
```

---

### `#[Lazy]`

Delays bean initialisation until the first `get_bean` call.

```rust
#[Component]
#[Lazy]
#[derive(Debug, Default, Clone)]
struct HeavyService { ... }
```

---

## `application.properties`

Place this file alongside your binary (or in the project root during `cargo run`). Values are loaded by `Application::run()` before any beans are wired.

Layering precedence (low -> high):

1. `application.properties`
2. `application-{profile}.properties` when `SPRING_PROFILE` is set
3. Environment overrides with `SPRING_PROP_*` (for example `SPRING_PROP_SERVER_PORT=9090`)

```properties
app.name=my-rust-app
server.port=9090
db.url=postgres://localhost/dev
```

---

## Running the Example

```bash
git clone https://github.com/arookieofc/rust-spring.git
cd rust-spring
cargo run -p example
```

Expected output:

```
[Singleton]  person bean: Person { id: 0, name: "" }
[Autowired]  user bean:   User { person: Person { ... }, id: 0, name: "" }
[Prototype]  requestContext: prototype bean (not cached in singleton store)
[Lazy]       heavyService: not yet initialized (lazy=true, needs do_create_bean)
[Lazy]       heavyService initialized: HeavyService { initialized: false }
[Bean]       appConfig: AppConfig { version: "1.0.0", max_connections: 100 }
[Value]      serverConfig: ServerConfig { port: 8080, app_name: "rust-spring", ... }
```

---

## Roadmap

- [x] IoC container (`BeanFactory`, `BeanDefinitionRegistry`)
- [x] Singleton & prototype scopes
- [x] Lazy initialisation
- [x] `#[autowired]` field injection
- [x] `#[Bean]` factory functions
- [x] `#[Value]` property injection from `application.properties`
- [x] AOP (aspect-oriented programming)
- [x] SpEL-style expression language
- [x] Conditional beans (`#[ConditionalOnProperty]`)
- [x] Spring Data-style repository abstraction
- [x] HTTP layer (Actix/Axum integration)

---

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).

## Security

See [SECURITY.md](SECURITY.md).

## Compatibility and migration

- Java compatibility matrix: [docs/java-spring-compat.md](docs/java-spring-compat.md)
- Migration notes: [docs/migration-from-spring.md](docs/migration-from-spring.md)

---

## License

This project is licensed under the [MIT License](LICENSE).
