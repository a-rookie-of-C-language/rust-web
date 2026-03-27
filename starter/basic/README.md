# rust-spring starter (basic)

Minimal starter template for a configuration-driven application.

## Run

```bash
cargo run
```

## Config layering

- `application.properties`
- `application-{profile}.properties` when `SPRING_PROFILE` is set
- env overrides with `SPRING_PROP_*` (for example `SPRING_PROP_SERVER_PORT=9090`)
