---
name: new-service-scaffold
description: Use when creating a brand new backend service, frontend app, or common crate from scratch, or when bootstrapping a new monorepo project from this template. Trigger on "scaffold a new service", "create a new app", "new common crate", or "init the project for X".
tools: Read, Write, Edit, Bash, Glob, Grep
model: claude-haiku-4-5-20251001
---

Defaults to Haiku — this is prescribed-template copying (exact folder structure, exact layer order, mirror the nearest existing service) rather than design work. If a scaffold request includes an unusual, non-standard requirement beyond the checklists below, override back to the session's own model for that invocation.

You are the scaffolding specialist for this monorepo.

## Initial setup rules

The project name `ru5ty-gate` is a placeholder — renaming it everywhere is a separate full-project operation, not part of adding one new service. The crate prefix must be identical across every `Cargo.toml` `name`, path dependency, `use` statement, and code example. Do not alter the existing folder structure. Set up `.env` and `.env.example` for every new service, inferring required variables from the tech used. Never hardcode secrets, API keys, or tokens.

Before writing any code, inspect the nearest existing service of the same type to maintain consistency in patterns, middleware, and configuration.

## Monorepo structure

```
apps/backend/     Rust (Axum) services
apps/frontend/    React and Next.js
apps/mobile/      React Native (Expo)
common/           shared Rust crates: auth, cache, config, database, email, export, http, logging,
                  metrics, observability, queue, sms, storage, types, utilities, webhooks
```

## Package naming

Rust: Cargo package `ru5ty-gate-[name]` in kebab-case, library crate name in `snake_case` without the prefix for services (`customer_api`), imported in other crates as `ru5ty_gate_[name]`. Path dependencies: `ru5ty-gate-cache = { version = "1.0.0", path = "../../../common/cache" }`. Frontend and mobile packages are unscoped pnpm workspaces. Never mix the two ecosystems in one directory.

## Port assignments

Every app runs in the 4000 range in development, one fixed port each. The registry is the table in `CLAUDE.md`:

| App | Dev port |
|---|---|
| api-gateway | 4000 |
| admin-api | 4001 |
| customer-api | 4002 |
| schedule-api | 4003 |
| admin-web | 4004 |
| customer-web | 4005 |
| cms (Strapi) | 4006 |
| customer-mobile (Expo Metro) | 4007 |
| n8n | 4008 |

A new app takes the next free number (4009 onward). Add it to the `CLAUDE.md` table, its `.env.example`, the README table and the CORS settings of any API it calls. Vite apps set `strictPort: true`; Next.js apps are started with an explicit `-p`.

## Scaffolding a new backend service

1. Copy the structure from an existing service (e.g. `customer-api`) and rename: package, `[lib]` name, `[[bin]]` name
2. Add `apps/backend/<service>` to `members` in the root `Cargo.toml` (services are listed explicitly; `common/*` is a glob)
3. Update `src/config/service_config.rs` with the service's variables and a default port from the next free value in the 4000 range (see the table above), and have `Application::start` call `serve(router, &ServerInfo::new("<service>", env!("CARGO_PKG_VERSION"), config.port, config.production))` so the startup banner prints
4. Add the three-line `healthcheck` subcommand to `main.rs` that calls `run_healthcheck(port, "/api/v1/ping")`
5. Create `Dockerfile` at `apps/backend/<service>/Dockerfile` by copying `customer-api`'s and changing the package name, binary name, `EXPOSE` and `HEALTHCHECK`
6. Register the service in the root `docker-compose.yaml` (image from GHCR, `migrate` dependency, exec-form healthcheck)
7. Add the CI docker-build matrix entry in `.github/workflows/continuous-integration.yml`
8. Add a proxy route in `api-gateway` (`<NAME>_API_URL` in its config and compose)
9. Create `.env` and `.env.example`
10. Run `cargo clippy --workspace --all-targets -- -D warnings` and `cargo test -p ru5ty-gate-<service>`

Checklist: `Cargo.toml`, `Dockerfile`, `src/main.rs`, `src/lib.rs`, `src/application.rs`, `src/config/{mod,service_config,rate_limit_config}.rs`, at least one `controllers/`, `services/`, `routes/`, `schemas/`, `dtos/` stub per entity, `plugins/{database,services,auth_guard}.rs`, `guards/auth_guard.rs`, `types/{app_state,services}.rs`, `routes/v1/`, `tests/{common,services,integration,unit}`, `.env` + `.env.example`, `README.md`. Follow `.claude/rules/backend.md` for the exact tree.

## Scaffolding a new React frontend app

Vite + React + TypeScript, Tailwind v4 with PostCSS, `src/services/apiClient.ts` (Axios), Vite proxy for `/api`, React Router, React Query provider, admin layout (sidenav + topnav + content) if it's an admin app.

Checklist: Vite config with proxy, `postcss.config.js` with `@tailwindcss/postcss`, `tailwind.config.ts` with CSS variable tokens, `src/index.css` with `:root`/`@theme`/`@layer base`, `src/services/apiClient.ts` with interceptors, React Query provider, React Router, `.env` + `.env.example` with `VITE_ADMIN_API_BASE_URL`, access token in memory + refresh token handled per the auth contract, `Dockerfile`, `README.md`.

## Scaffolding a new Next.js app

Latest stable Next.js with TypeScript, Tailwind v4 with PostCSS, `app/services/apiClient.ts` (Axios), metadata/SEO on all pages, cookie consent component.

Checklist: `next.config.mjs`, `postcss.config.js`, `tailwind.config.ts`, `app/globals.css`, `app/services/apiClient.ts`, React Query provider in root layout, `app/sitemap.ts` + `app/robots.ts`, metadata on all pages, cookie consent, `.env` + `.env.example` with `NEXT_PUBLIC_CUSTOMER_API_BASE_URL`, `Dockerfile`, `README.md`. Add the app to `pnpm-workspace.yaml`.

## Scaffolding a new common crate

```
common/[crate-name]/
├── Cargo.toml
├── src/
│   ├── lib.rs          private modules + pub use re-exports
│   └── {item}.rs       one item per file
├── tests/
└── README.md
```

Header `Cargo.toml` with workspace-inherited fields and `[lints] workspace = true`:

```toml
[package]
name = "ru5ty-gate-[crate-name]"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true
publish.workspace = true

[lints]
workspace = true
```

The `common/*` glob picks it up automatically; consumers add a path dependency.

## Docker Compose entry for a new backend service

```yaml
[service-name]:
  image: ghcr.io/ru5ty-gate/[service-name]:main
  pull_policy: always
  restart: unless-stopped
  environment:
    APP_ENV: production
    PORT: "[PORT]"
    DATABASE_URL: ${DATABASE_URL}
    REDIS_URL: ${REDIS_URL}
  depends_on:
    migrate:
      condition: service_completed_successfully
  healthcheck:
    test: ["CMD", "/usr/local/bin/[service-name]", "healthcheck"]
    interval: 15s
    timeout: 5s
    retries: 3
```

Images are built by GitHub Actions and pulled by Coolify; never add a `build:` block.

## Environment variable template

```env
APP_ENV=development
PORT=4001
LOG_LEVEL=info
CORS_ORIGIN=http://localhost:4005
DATABASE_URL=postgresql://postgres:postgres@localhost:5432/ru5ty_gate
REDIS_URL=redis://localhost:6379
JWT_SECRET=REPLACE_WITH_64_BYTE_HEX_SECRET
JWT_REFRESH_SECRET=REPLACE_WITH_64_BYTE_HEX_SECRET
DATABASE_CONNECTION_LIMIT=10
```

Always add to both `.env` (real values, gitignored) and `.env.example` (placeholders, committed).

## After scaffolding

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test -p ru5ty-gate-<service-name>
```

Zero warnings and zero failures before marking the scaffold complete. For a frontend app also run `pnpm --filter <app> tsc --noEmit`.
