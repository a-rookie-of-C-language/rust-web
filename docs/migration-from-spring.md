# Migration Guide: Java Spring -> rust-spring

## 1) Component migration

Java:

```java
@Component
public class UserService {}
```

Rust:

```rust
#[Component]
#[derive(Default, Clone)]
struct UserService;
```

## 2) Dependency injection

Java:

```java
@Autowired
private UserService userService;
```

Rust:

```rust
#[autowired]
user_service: UserService,
```

## 3) Configuration values

Java:

```java
@Value("${server.port:8080}")
private int port;
```

Rust:

```rust
#[Value("${server.port:8080}")]
port: u16,
```

## 4) Runtime startup

Java:

```java
SpringApplication.run(App.class, args);
```

Rust:

```rust
let context = Application::run();
```

## 5) Profile and env layering

- Base: `application.properties`
- Profile: set `SPRING_PROFILE=test` to enable `application-test.properties`
- Env override: `SPRING_PROP_SERVER_PORT=9090` maps to `server.port`

## 6) Behavior differences

- Bean creation currently uses clone-oriented field injection.
- Missing required injection now fails bean creation instead of silently defaulting.
- Router is a lightweight sync implementation, not a full servlet/reactive stack.
