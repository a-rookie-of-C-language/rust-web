# Security Policy

## Supported Versions

The latest `main` branch is supported for security fixes.

## Reporting a Vulnerability

- Do not open public issues for undisclosed vulnerabilities.
- Send a report to `2128194521hzz@gmail.com` with:
  - affected crate(s) and version/commit
  - reproduction steps
  - impact assessment
  - any suggested mitigation

You can expect an acknowledgement within 72 hours and a remediation plan after triage.

## Dependency Auditing

This repository runs `cargo audit` in CI. Contributors should also run:

```bash
cargo install cargo-audit --locked
cargo audit
```
