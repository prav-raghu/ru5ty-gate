# ru5ty-gate

## Instruction precedence

When guidance conflicts, resolve in this order:

1. The user's explicit request
2. Security/permission constraints in `.claude/settings.json`
3. Current repository code and committed configuration (e.g. `docker-compose.yaml`, `common/database/migrations`) — a written example never overrides what's actually committed
4. Path-specific files in `.claude/rules/`
5. The selected subagent in `.claude/agents/`
6. This file
7. Deep-dive reference docs in `.claude/instructions/`
8. Legacy commands in `.claude/commands/` — thin entry points that delegate to an agent; treat any command content that contradicts a higher-precedence source as stale, not authoritative

If a command, agent, and rule genuinely disagree instead of one being simply out of date, say so and ask rather than silently picking one.

## Subagents (available; delegate when a task clearly matches a description)

Descriptions below help Claude choose the right delegate — they are not a deterministic routing table. When a task matches one clearly, use it; when it's ambiguous, pick the closest fit or ask.

| Subagent | Scope |
|---|---|
| `full-stack-orchestrator` | Builds spanning database + backend + frontend together |
| `backend-service` | General Axum service work (controllers, routes, middleware, guards) |
| `api-builder` | Generating full CRUD layers from an existing database table and model |
| `jwt-security` | Scaffolding/reviewing auth routes (login, logout, refresh) — token denylist, refresh rotation, cookie config. Apply alongside `api-builder` for auth domains |
| `domain-modeler` | Designing new tables and Rust models from business requirements |
| `relational-database` | SQLx operations — migrations, seeding, naming, database issues |
| `database-migrations` | Zero-downtime migration patterns, backfills, CI migration checklist |
| `audit-log` | Audit trail pattern for state-changing operations |
| `enterprise-scale` | Cross-cutting 1M+ concurrent user patterns — cache, queue, pagination |
| `frontend-react` | Admin-web (React + Vite SPA) |
| `frontend-nextjs` | Customer-web (Next.js, SEO/SSR) |
| `frontend-page-builder` | Generating full page/component/hook layers for a domain |
| `mobile` | React Native (Expo) customer-mobile app |
| `common-packages` | Shared `common/*` crates (database, cache, config, logging, etc.) |
| `new-service-scaffold` | Scaffolding a brand new service, app, or crate |
| `rbac` | Permissions, role guards, role-to-permission mapping |
| `webhook-events` | Outbound webhooks and the internal event bus |
| `feature-flags` | DB-backed feature flag store and evaluation |
| `infrastructure` | Terraform, Kubernetes (future), Docker Compose, NGINX |
| `vps-bootstrap` | One-time fresh-VPS setup — prerequisite to deployment-coolify |
| `deployment-coolify` | Canonical deploy path: Coolify, Dockerfile builds, managed Postgres/Redis, DNS |
| `testing` | Rust service/integration tests (`#[sqlx::test]`), factories, frontend test setup |
| `rust-standards` | Rust idiom, lint and type-safety review outside a full code review |
| `typescript-standards` | Type-safety review for frontend and mobile TypeScript |
| `code-review` | Full quality/security audit |

For anything not covered by a subagent above, read the relevant file in `.claude/instructions/` before writing code.

## Model selection when delegating to a subagent

Every subagent's `model:` frontmatter is `inherit` unless noted below — it runs on whatever model this session is running on. The `Agent` tool also accepts a per-call `model` override that beats the agent's own frontmatter; use it deliberately to control cost, not by default.

**Override down to `claude-haiku-4-5-20251001` only when *all* of these hold:**
- The task mirrors an existing, unambiguous pattern already in this codebase (e.g. "add a 6th CRUD entity shaped exactly like the other 5") — not a first-of-its-kind design decision.
- Nothing security-, auth-, RBAC-, payment-, or PII-adjacent is being written or touched.
- A mistake would be caught by `cargo clippy`/`tsc`/lint/tests before merge — not the kind of subtle logic error that slips past mechanical checks and only shows up as a real bug later.
- The blast radius is one file or one entity, not a shared `common/*` crate or a cross-cutting concern.

`new-service-scaffold` and `testing` (for straightforward, already-understood service/controller test-writing — not for designing a test strategy for something novel) default to Haiku in their own frontmatter for exactly this reason: they're closer to templating than judgment. That default is a starting point, not a floor — bump either back to Sonnet for an unusually complex instance of their normal work.

**Never override down — keep Sonnet (or the session's own model) — for:** `jwt-security`, `rbac`, `database-migrations`, `domain-modeler`, `code-review`, `rust-standards`, `typescript-standards`, `deployment-coolify`, `infrastructure`, `vps-bootstrap`, `enterprise-scale`, `full-stack-orchestrator`, `audit-log`, `webhook-events`, `common-packages`, `relational-database`. These carry either security consequences, wide blast radius, or genuine architectural judgment that a mistake won't surface as a clean typecheck failure — it surfaces as a production incident or a silent vulnerability. `code-review` in particular is the backstop for everything else in this list; downgrading the backstop defeats the point of having one.

Everything else (`api-builder`, `frontend-page-builder`, `frontend-react`, `frontend-nextjs`, `mobile`, `feature-flags`, `backend-service`) is task-dependent — judge the specific request against the four bullets above each time rather than a fixed per-agent answer.

This list is a starting point, not a settled policy — if a Haiku-delegated task comes back needing real rework, that's a signal to tighten these criteria (or move that agent to the "never override" list), not to push through it.

## Deployment — Coolify is canonical

This project deploys to a self-hosted Hetzner VPS via Coolify. `vps-bootstrap` runs once on a fresh server (and installs Coolify); `deployment-coolify` covers everything after that (Dockerfiles, per-application config, env vars, managed Postgres/Redis, git-push auto-deploy, DNS). No other deployment path (Render, raw SSH, Kamal) is used for this project.

## Skills (invoke with /name, or loaded by Claude when a description clearly matches)

| Skill | When to use |
|---|---|
| `/ui-ux-pro-max` | Before building any frontend page or component — design system lookup, color, typography, UX patterns |
| `/build-page` | Build a complete frontend page end-to-end with design intelligence baked in |
| `/impeccable` | After a page/component is built — 23-command design taste pass (`critique`, `audit`, `polish`, `bolder`, `quieter`, `distill`, `animate`, ...) plus a deterministic 59-rule anti-"AI slop" detector (gradient text, purple/violet gradients, glowing dark-mode accents, overused fonts, WCAG contrast, bounce easing) |
| `emil-design-eng` | Animation and micro-interaction review — Emil Kowalski's (Sonner/Vaul author) design-engineering rules: keep UI animations under 300ms, never `ease-in` for entrances, custom easing over CSS defaults, spring physics, before/after review tables |
| `design-taste-frontend` (`taste-skill`) | Anti-slop pass for **marketing/landing pages, portfolios, and redesigns only** — explicitly not scoped for dashboards, data tables, or multi-step product UI, so skip it for admin-web CRUD screens. Tunable via `DESIGN_VARIANCE`/`MOTION_INTENSITY`/`VISUAL_DENSITY` (1–10) |
| `/security-review` | Audit code for auth gaps, injection risks, and secrets before merge |
| `/code-review-skill` | Full quality review: types, naming, security, form validation coverage |
| `/seo-optimization` | Audit/improve SEO for `customer-web` (Next.js) — metadata, structured data, sitemaps, Core Web Vitals |

### Design taste stack

`ui-ux-pro-max` and `build-page` remain the primary design authority for this monorepo — design-system lookup, palettes, fonts, and stack-specific component patterns for the actual build. `impeccable`, `emil-design-eng`, and `design-taste-frontend` are an **additive taste/anti-slop layer** run after a page or component is built, not a replacement:

- **`impeccable`** is the general-purpose one — works for any surface (dashboards included). Run `/impeccable audit <target>` or `/impeccable polish <target>` as a finishing pass on new frontend work.
- **`emil-design-eng`** narrows to motion/animation review specifically — invoke when a page has non-trivial transitions, loading states, or micro-interactions worth scrutinizing.
- **`design-taste-frontend`** only fits customer-web marketing/landing surfaces (its own description explicitly excludes dashboards and data tables) — do not reach for it on admin-web.

Vendored from `pbakaus/impeccable`, `emilkowalski/skills`, and `Leonxlnx/taste-skill` respectively (Apache-2.0 / MIT / MIT) — see each skill folder's `SOURCE.md` for the commit vendored and how to refresh it. `impeccable`'s own PostToolUse/Stop hook auto-run was **not** wired into `.claude/settings.local.json` — it's available to invoke manually via `/impeccable ...`; opt into the automatic per-edit hook yourself if you want it (see `.claude/skills/impeccable/reference/hooks.md`).

### Meta — Skill Creator

`skill-creator` (global, not project-local) scaffolds a new reusable skill from a plain-English description — testing, packaging, and description-tuning included. Reach for it when a multi-step task pattern has come up 3+ times in this project and would benefit from a repeatable, invocable skill instead of re-explaining it each time (mirrors the existing skills above, which were built the same way). It's the capability-uplift complement to the domain-specific skills in the table: those encode *this project's* conventions, `skill-creator` is how you add the next one without hand-authoring `SKILL.md` frontmatter from scratch.

## Commands (legacy, still work)

`/add-endpoint`, `/add-entity`, `/add-service`, `/add-pages`, `/design-database`, `/add-tests`, `/review`, `/build-system`, `/deploy-coolify`, `/init-project`, `/provision-infrastructure`, `/request-logging`, `/strapi-setup`

## Memory — Claude Mem (replaces Serena)

Serena (LSP-backed code navigation MCP server) has been dropped from this project — no `.mcp.json` and no `.serena/` config are committed anymore. In its place, use **Claude Mem** for cross-session memory: it hooks into the session lifecycle (`SessionStart`, `UserPromptSubmit`, `PostToolUse`, `Stop`, `SessionEnd`), summarizes what happened, and stores it in a local SQLite + vector-search store so decisions and context persist between sessions instead of vanishing when a session ends.

This is a per-developer, one-time install — Claude does not run it for you:

```bash
npx claude-mem install
```

Local dashboard (session history, memory search): `http://localhost:37777`. Wrap anything session-specific and sensitive (API keys, customer data) in `<private>...</private>` in a prompt to exclude it from what gets stored. Treat Claude Mem as the project's answer to "does this session remember what the last one decided" — reach for it instead of re-explaining architectural decisions each session.

## UI/UX skill setup (one-time global install)

```bash
npm install -g ui-ux-pro-max-cli
uipro init --ai claude --global
```

Requires Python 3.x. After install, the `/ui-ux-pro-max` skill has access to 50+ design styles, 161 color palettes, 57 font pairings, and stack-specific guidance for React, Next.js, and React Native.

## Non-negotiable rules

- Backend is Rust (edition 2024, toolchain pinned in `rust-toolchain.toml`) on Axum 0.8 + Tokio — no Node.js backend code, no Actix/Rocket/Warp
- `validator` derives plus `#[serde(deny_unknown_fields)]` only for backend validation — never hand-rolled checks in controllers, never Zod on the backend
- Rust is strict: `unsafe` is forbidden, and `unwrap`, `expect`, `panic!`, `todo!`, `unimplemented!`, `dbg!` are denied by clippy outside tests (tests opt out with an explicit `#![allow(...)]`)
- Frontend/mobile TypeScript strict — no `any`, no `as unknown as T`, no `@ts-ignore`
- DTOs, schemas, models and types are plain structs/enums — never behaviour-heavy objects
- No comments in code
- No hardcoded secrets — all secrets via environment variables
- Folder structure is immutable — do not create new top-level folders
- `common/` for all shared crates and packages — not `packages/` or `libs/`
- Rust crates are named `ru5ty-gate-[name]` in Cargo and imported as `ru5ty_gate_[name]`; frontend packages stay unscoped
- All structs, enums, traits, DTOs and constants in their own files; `mod.rs`/`lib.rs` re-export
- Match existing coding style — controllers are unit structs with async associated functions, services are structs with `impl` blocks
- SQL lives in the repository layer (`common/database/src/repositories/`), not inline in services: the generic `Repository::<Model>` for plain CRUD and `{Entity}Repository` unit structs for anything table-specific. New data access uses it from the start; services that still hold inline SQL are migrated when they are touched — see `rules/backend.md` (SQL access) and `documentation/repository-layer.md`
- All files use LF line endings (`.gitattributes` sets `eol=lf`; `rustfmt.toml` and `.editorconfig` agree). Never commit CRLF
- Do not run database migrations — the developer runs them unless explicitly asked (`admin-api migrate`, `sqlx migrate`)
- Do not run git operations — the developer runs them unless explicitly told to
- Service layer of every API needs automated tests in `tests/services` (real database via `#[sqlx::test]`); write frontend unit tests too where feasible
- Dockerfiles live at each app's own root: `apps/backend/<service>/Dockerfile`, `apps/frontend/<app>/Dockerfile`
- Root `docker-compose.yaml` is the single Coolify deployment stack
- Before marking any Rust task complete, run `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings` and `cargo test -p <crate>` — zero warnings and zero failures required
- Before marking any frontend TypeScript task complete, run `pnpm --filter <app> tsc --noEmit` — zero errors required
- All frontend forms must have validation: required fields, email format, phone format, and any other applicable constraints
- Frontend env vars are scoped per app: `VITE_<SCOPE>_*` (Vite) / `NEXT_PUBLIC_<SCOPE>_*` (Next.js, client-exposed) — see `rules/frontend.md` for the full convention and why `NEXT_PUBLIC_` can't be substituted
- Backend environment is read once into a typed `ServiceConfig` through `EnvReader`; `APP_ENV` selects `development`/`production`
- Redis connections use `REDIS_URL` only — no discrete `REDIS_HOST`/`REDIS_PORT`/`REDIS_PASSWORD` fallback anywhere in the stack
- Database connections use `DATABASE_URL` only — no discrete `DB_HOST`/`DB_PORT`
- The first `SUPER_ADMIN` account is created only through `admin-api`'s one-time `/auth/bootstrap-admin` route (gated by `ADMIN_BOOTSTRAP_ENABLED`) — see `jwt-security.md`'s "Admin bootstrap" section. Never add a general-purpose "create admin" path outside it.
- Every table gets the six base metadata columns (`id`, `is_active`, `created_at`, `updated_at`, `created_by`, `modified_by`) unless it's a narrow, documented exception — see `rules/database.md`
- Dates: `TIMESTAMPTZ`/UTC in the DB, ISO 8601 on the wire in both directions, `dd/MM/yyyy` (+ optional `HH:mm:ss`) on screen via `date-fns` — see `date-handling.instructions.md`
- Email fields get disposable-domain rejection in the service layer, not just `validator` syntax checks — see `validation-chain.instructions.md`
- Logout invalidates every active session for that user (all devices/tabs), not just the token that called logout — see `jwt-security.md`'s per-user `minIat` marker pattern
- MFA (TOTP) is two-step at login when `twoFactorEnabled`: password success returns a short-lived `mfaToken`, not real tokens — real tokens issue only after `/auth/verify-login-mfa` passes. Never treat password-correct as login-complete for an MFA-enabled user. See `jwt-security.md`'s "MFA / Two-Factor Authentication" section
- No god structures: never name a controller, service, DTO, or file after the project/app itself (e.g. `ranga_controller.rs`, `admin_service.rs`) as a catch-all that encapsulates every entity's logic. Each entity gets its own service and DTOs under its own file, named after the entity (`user_service.rs`, `user_dto.rs`). A controller may still be entity-scoped (`users_controller.rs`) or composed into a dashboard/aggregate controller that calls into per-entity services — the composition happens at the controller/route layer, never by collapsing entity logic into one shared file. See `.claude/agents/backend-service.md` and `.claude/agents/api-builder.md` for the controller/service/DTO file layout this produces.

## Quality gates

Three checks are non-negotiable before code is considered mergeable, and each has a Rust and a frontend half that run together through the root scripts:

| Gate | Rust (backend + `common/`) | TypeScript (frontend, mobile) | Root script |
|---|---|---|---|
| **typecheck** | `cargo check --workspace --all-targets` | `tsc --noEmit` | `pnpm typecheck` |
| **lint** | `cargo clippy --workspace --all-targets -- -D warnings` | ESLint | `pnpm lint` |
| **format** | `cargo fmt --all --check` | Prettier | `pnpm format:check` |

These are the full quality-gate set for this project — do not add or substitute other gates (coverage thresholds, bundle size, etc.) as if they were part of this baseline; those remain separate, optional checks. `cargo deny check` (licences, advisories, bans per `deny.toml`) runs in CI as a supply-chain check alongside them.

- **Pre-commit hook** (`.husky/pre-commit` → `lint-staged.config.mjs`): staged `*.rs` files run `rustfmt` and `cargo clippy`; staged frontend files run Prettier and ESLint. A commit that fails typecheck or lint should be blocked locally, not caught later in CI.
- **GitHub Actions** (`.github/workflows/continuous-integration.yml`): the `lint`, `typecheck`, and `quality-gate` job (which gates on `lint`, `typecheck`, `build`, `test`) must pass before the `docker-build` matrix job and the subsequent `deploy` job execute. The `test` job runs Postgres 16 and Redis 7 service containers for the Rust tests. Same applies to `pull-request-checks.yml` for PR-time checks.
- Do not weaken either gate (e.g. `continue-on-error: true` on lint/typecheck, `#![allow(...)]` on production code to silence clippy, or excluding files from lint-staged) without the developer explicitly asking for it.

## Folder structure (immutable)

```text
apps/backend/        Rust (Axum) services — one Cargo crate per service
apps/frontend/       React (admin-web) and Next.js (customer-web)
apps/mobile/         React Native (Expo)
apps/cms/            Strapi (managed independently with npm)
apps/automation/     n8n
common/              shared Rust crates (and any shared frontend packages) only
devops/              local dev Docker Compose and scripts
infrastructure/      Nginx, Terraform
documentation/       markdown docs
.github/             CI/CD workflows
.claude/             Claude Code configuration
  agents/            subagent definitions (delegated to by description match, not deterministic)
  commands/          legacy slash commands (single-file)
  hooks/             scripts run on tool use events
  instructions/      reference docs, read manually or by cross-reference — not auto-loaded (see note below)
  rules/             path-gated rules (auto-load when matching files enter context via `paths:` frontmatter)
  skills/             reusable skills invoked with /name
  templates/         scope and PR templates
  workflows/         dynamic multi-agent workflow scripts
```

`.claude/instructions/` predates `.claude/rules/` and is not auto-gated by Claude Code itself — any `applyTo:` frontmatter on files in that folder is informational only, not a mechanism the harness parses. Domain conventions that need to auto-load on matching files live in `.claude/rules/` instead; `.claude/instructions/` is now reserved for deep-dive reference docs (e.g. the SQL → `validator` → frontend Zod validation chain, JWT token lifecycle, OpenAPI setup) that agents and rules link to by name.

## Package managers

- **Backend and `common/`**: Cargo. The Rust workspace is the root `Cargo.toml` with `Cargo.lock` committed; crates inherit lints, edition and shared dependency versions from `[workspace]`. Add dependencies with `cargo add -p <crate>` and prefer `workspace = true` for shared ones.
- **Frontend, mobile and root tooling**: pnpm — always use `pnpm`, never `npm` or `yarn`. Internal deps use `workspace:*`. `pnpm-workspace.yaml` lists the frontend and mobile apps only; it no longer includes backend or `common/`.

## Port assignments

**Non-negotiable: every app runs in the 4000 range in local development**, one fixed port each, listed below. Never move a dev port outside the range, never reuse a port, and never let a dev server pick another port automatically (the Vite apps set `strictPort`, Next.js is started with an explicit `-p`, Expo with `--port 4007`). When a new app is added, take the next free number and add it to this table, the app's `.env.example`, the README and the CORS settings of any API it calls. Production containers keep their own ports (right column) and are unaffected.

| App | Dev port | Container / production port |
|---|---|---|
| api-gateway | 4000 | 4000 |
| admin-api | 4001 | 4001 |
| customer-api | 4002 | 4002 |
| schedule-api | 4003 | 4003 |
| admin-web (Vite) | 4004 (`VITE_ADMIN_PORT`, see `vite.config.ts`) | 80 (nginx, Traefik-routed) |
| customer-web (Next.js) | 4005 (`next dev -p 4005`) | 3000 |
| cms (Strapi) | 4006 (`PORT`) | 4006 |
| customer-mobile (Expo / Metro) | 4007 (`expo start --port 4007`) | not deployed as a container |
| n8n | 4008 (host port of the dev compose, container listens on 5678) | per-project instance |
| agent (captive portal) | 4009 (`bind_addr` in `apps/backend/agent/config/agent.toml`) | 2080 on the router, not a container |

`CORS_ORIGIN` accepts a comma-separated list of origins, for APIs called from more than one app (for example two web origins). Native mobile requests send no browser origin, so CORS does not apply to the Expo app.

## Project: Ru5ty Gate

Captive portal platform. A Rust agent runs on the venue router (GL-iNet GL-MT6000, OpenWrt) behind openNDS as its Forwarding Authentication Service, and a central platform built from the standard services in this monorepo sits behind it. The structure comes from the Rust monorepo template; the namespace is `ru5ty-gate` / `ru5ty_gate`. The code is proprietary to Zynkosi Tech (Pty) Ltd. (see `LICENSE`); workspace crates declare `LicenseRef-Zynkosi-Proprietary`.

### Agent and its crates

- `apps/backend/agent` is the router daemon (`ru5ty-gate-agent`, binary `ru5ty-gate-agent`). Its README holds the protocol, configuration and openNDS setup.
- The pieces it wires together are shared crates under `common/`: `agent-config` (TOML settings), `session-store` (sqlite sessions and sync queue), `central-client` (central API HTTP client), `fas-server` (openNDS FAS routes) and `heartbeat` (heartbeat and sync tasks).
- Local development uses `config/agent.example.toml` (port 4009, database under `apps/backend/agent/data/`). `config/agent.router.example.toml` holds the router settings (port 2080). `agent.toml` and `data/` are git-ignored.

### Deliberate exceptions for the agent

The agent runs on a router, so these rules from the sections above do not apply to it and its crates:

- Configuration is a TOML file through `ru5ty-gate-agent-config`, not `EnvReader` and `ServiceConfig`.
- Storage is sqlite through `rusqlite`, not Postgres: no SQLx migrations, no `common/database` repository layer, no `DATABASE_URL`.
- No Dockerfile, no entry in `docker-compose.yaml`, no Coolify application and no CI docker-build matrix entry. The binary is cross-compiled for OpenWrt.
- `rusqlite` stays on the 0.39 line. `sqlx` resolves `libsqlite3-sys` 0.37 and Cargo allows one crate linking the native sqlite library, so a newer `rusqlite` fails resolution.

Every other rule applies unchanged, including strict clippy, no comments, one item per file and tests in `tests/`.

### Central platform

The endpoints the agent calls (`/v1/venues/{venue_id}/sessions/validate`, `/policy`, `/heartbeat`, `/sync`) are documented in `common/central-client/README.md`. They are not implemented in `admin-api`, `customer-api` or `api-gateway` yet.

### Open security findings and feature gaps

`documentation/security-and-feature-review.md` is the work queue from the October 2026 review: security findings (`SEC-*`) and missing features (`FEAT-*`), each with location, evidence, fix and acceptance criteria, in a suggested order. Work through it one item per commit and tick its Status table as items merge.

### Region-specific defaults

The template was built for a South Africa-based project and this project keeps those defaults: the phone validation regex `^(\+27|0)[6-8][0-9]{8}$` and SMSPortal for SMS. Hetzner EU (Falkenstein) in `vps-bootstrap.md` and `deployment-coolify.md` remains a starting point rather than a requirement.
