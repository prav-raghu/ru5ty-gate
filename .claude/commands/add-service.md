---
description: Add a complete new backend service to the monorepo with all boilerplate
argument-hint: <service name and purpose, e.g. "notification-api for push notifications and email alerts">
---

Scaffold a new backend service: $ARGUMENTS

Use the `new-service-scaffold` subagent and match `customer-api`'s structure exactly — do not introduce new patterns. Before writing any code, read the existing `customer-api` to understand current plugin registration order, config patterns, and route setup.

1. Create the full directory structure under `apps/backend/{service-name}/`
2. Copy and adapt patterns from `customer-api`: `main.rs` (with the `healthcheck` subcommand), `lib.rs`, `application.rs`, `config/service_config.rs` (typed `EnvReader` config), the middleware stack order, `plugins/{database,services,auth_guard}.rs`, v1-prefixed routes, `types/{app_state,services}.rs`, `tests/`, and `Cargo.toml`; add the crate to `members` in the root `Cargo.toml`
3. Assign the next available port (check existing: 4000, 4001, 4002, 4003)
4. Create a `Dockerfile` at `apps/backend/{service-name}/Dockerfile` using the template from the `deployment-coolify` agent
5. Add to root `docker-compose.yaml`
6. Add a proxy route in `api-gateway`
7. Create `.env` and `.env.example`
8. Add the service to `devops/scripts/dev.sh` and the CI docker-build matrix
9. Run `cargo fmt --all --check` and `cargo clippy --workspace --all-targets -- -D warnings` — zero warnings required
10. Run `cargo test -p ru5ty-gate-{service-name}`
