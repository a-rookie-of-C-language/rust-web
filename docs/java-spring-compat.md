# Java Spring Compatibility Matrix

## Core Container

| Java Spring | rust-spring | Status |
|---|---|---|
| `@Component` | `#[Component]` | Supported |
| `@Autowired` | `#[autowired]` | Supported |
| `@Scope` | `#[Scope("singleton/prototype")]` | Supported |
| `@Lazy` | `#[Lazy]` | Supported |
| `@Value` | `#[Value("${...}")]/SpEL` | Supported |
| `@ConditionalOnProperty` | `#[ConditionalOnProperty(...)]` | Supported |
| `@Qualifier` | N/A | Planned |
| `@Primary` | N/A | Planned |

## Boot & Config

| Java Spring Boot | rust-spring | Status |
|---|---|---|
| `SpringApplication.run` | `Application::run()` | Supported |
| `application.properties` | `application.properties` | Supported |
| profile files | `application-{profile}.properties` | Supported |
| env override | `SPRING_PROP_*` | Supported |

## AOP & Web

| Java Spring | rust-spring | Status |
|---|---|---|
| `@Aspect` | `#[Aspect]` | Supported |
| `@Before/@After/@Around` | same names | Supported |
| `@GetMapping/...` | `#[GetMapping]/...` | Supported |
| Full middleware stack | minimal router dispatch | Partial |

## Data

| Java Spring Data | rust-spring | Status |
|---|---|---|
| Repository abstraction | `Repository<T>` | Supported |
| in-memory implementation | `InMemoryRepository<T>` | Supported |
| DB adapters | N/A | Planned |
