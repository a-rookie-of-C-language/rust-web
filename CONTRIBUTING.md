# Contributing to rust-spring

Thank you for considering contributing! This document explains how to get involved.

English | [中文](CONTRIBUTING.zh.md)

---

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [How to Contribute](#how-to-contribute)
- [Development Setup](#development-setup)
- [Commit Convention](#commit-convention)
- [Pull Request Process](#pull-request-process)
- [Reporting Bugs](#reporting-bugs)
- [Suggesting Features](#suggesting-features)

---

## Code of Conduct

Be respectful. Constructive criticism is welcome; personal attacks are not.

---

## Getting Started

1. **Fork** the repository on GitHub.
2. **Clone** your fork locally:
   ```bash
   git clone https://github.com/a-rookie-of-C-language/rust-spring.git
   cd rust-spring
   ```
3. **Check that everything builds:**
   ```bash
   cargo build --workspace
   cargo test --workspace
   ```

---

## How to Contribute

| Type | Where to look |
|---|---|
| Bug fix | Open an issue first, then a PR |
| New annotation / feature | Open an issue to discuss design first |
| Documentation improvement | PR directly — no issue needed |
| Refactor / cleanup | PR with a clear explanation |

---

## Development Setup

### Requirements

- Rust stable toolchain (`rustup update stable`)
- `rustfmt` and `clippy` (bundled with `rustup`):
  ```bash
  rustup component add rustfmt clippy
  ```

### Useful commands

```bash
# Build everything
cargo build --workspace

# Run all tests
cargo test --workspace

# Check formatting
cargo fmt --all -- --check

# Apply formatting
cargo fmt --all

# Run linter (must pass with zero warnings)
cargo clippy --workspace --all-targets -- -D warnings

# Security audit
cargo install cargo-audit --locked
cargo audit

# Run the example
cargo run -p example

# Generate a fresh demo project
cargo run -p initializer -- --name demo --output /tmp
```

### Proc-macro UI contracts (trybuild)

`spring-macro` includes a trybuild harness at `spring-macro/tests/trybuild.rs`.
It is currently marked ignored until stderr snapshots are finalized.

### Project layout

```
spring-core        Core traits and abstractions
spring-beans       BeanFactory, BeanDefinition, Environment, PropertySource
spring-context     ApplicationContext, bean lifecycle management
spring-boot        Public entry point — re-exports everything users need
spring-macro       Proc-macro crate: #[Component], #[Bean], #[Value], etc.
spring-aop         AOP module (stub, in progress)
spring-expression  Expression engine (stub, in progress)
spring-util        Shared helper utilities
example            Integration demo — always kept runnable
initializer        CLI project scaffolding tool
```

---

## Commit Convention

Use [Conventional Commits](https://www.conventionalcommits.org/):

```
<type>(<scope>): <short description>
```

| Type | When to use |
|---|---|
| `feat` | New feature or annotation |
| `fix` | Bug fix |
| `refactor` | Code change without behaviour change |
| `docs` | Documentation only |
| `test` | Adding or fixing tests |
| `chore` | Build scripts, CI, dependencies |
| `ci` | CI/CD configuration |

Examples:
```
feat(macro): add #[ConditionalOnProperty] annotation
fix(beans): resolve singleton cache miss on first get_bean call
docs(readme): add #[Scope] example
```

---

## Pull Request Process

1. **Branch** from `main`:
   ```bash
   git checkout -b feat/my-feature
   ```
2. Make your changes. Keep commits atomic.
3. Ensure **all checks pass locally**:
   ```bash
   cargo fmt --all -- --check
   cargo clippy --workspace --all-targets -- -D warnings
   cargo test --workspace
   ```
4. **Open a PR** against `main`. Fill in the PR template.
5. At least one review approval is required before merging.
6. Squash commits if the history is noisy.

---

## Reporting Bugs

Open a [GitHub Issue](https://github.com/arookieofc/rust-spring/issues) with:

- **Rust version** (`rustc --version`)
- **Minimal reproducible example**
- **Expected behaviour**
- **Actual behaviour** (include the full error message / panic output)

---

## Suggesting Features

Open a [GitHub Issue](https://github.com/arookieofc/rust-spring/issues) labelled `enhancement`. Describe:

- **The problem** you are trying to solve
- **Your proposed solution**
- **Alternatives** you considered

Large features (new crates, new annotation semantics) should be discussed in an issue before any code is written.

---

## Contact

Maintainer: **arookieofc** — [2128194521hzz@gmail.com](mailto:2128194521hzz@gmail.com)
