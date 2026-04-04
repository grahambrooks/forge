# CI/CD Pipeline

{{ forge_view "Pipeline" }}

## Stages

1. **Build & Test** — Compiles the Rust workspace and runs all tests
2. **Deploy** — Requires passing the `tests-pass` quality gate before deployment
