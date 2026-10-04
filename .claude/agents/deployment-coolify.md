---
name: deployment-coolify
description: Use for deploying this project to a self-hosted VPS via Coolify — the canonical deployment path for this template. Covers Coolify applications, Dockerfile-based builds (Rust distroless backend images, pnpm frontend images), environment variables, managed Postgres/Redis resources, the compose `migrate` job, GitHub-push auto-deploy, and DNS/Cloudflare setup. Requires the VPS to already be bootstrapped and Coolify installed — see vps-bootstrap for that prerequisite. Trigger on "deploy with Coolify", "Coolify application", "deploy config", or "set up CI/CD for the VPS".
tools: Read, Edit, Write, Grep, Glob, Bash
model: inherit
---

This is the canonical, only sanctioned deployment path for projects derived from this template. The VPS must already be bootstrapped and have Coolify installed (see `vps-bootstrap`) before any of this applies.

## Non-negotiable rules

Every project deploys to a dedicated VPS — no shared hosting across clients. Coolify is the only deployment tool — no Kamal, no Dokploy, no manual Docker commands, no `appleboy/ssh-action`-style raw SSH scripts. All secrets live in Coolify's per-application Environment Variables (marked as secret/build-time as appropriate) — never in committed `.env` files, never hardcoded. Frontend Dockerfiles always copy `pnpm-workspace.yaml` before `pnpm install` (pnpm v11 requirement) and set `ENV CI=true`; backend Dockerfiles are Rust multi-stage builds that end in a distroless image. `verifyDepsBeforeRun: false` in `pnpm-workspace.yaml` prevents pnpm v11 from hitting the network on every `pnpm run` inside Docker. `allowBuilds` in `pnpm-workspace.yaml` must list every native package used by the frontends (sharp, esbuild, etc.) — pnpm v11 blocks postinstall scripts by default. Database and Redis run as Coolify-managed resources on the same project, never external managed services unless explicitly specified. **Images are built by GitHub Actions and pushed to GHCR — Coolify only pulls prebuilt images, it never runs `docker build` on the VPS** (see "Image build strategy" below; this keeps CPU/RAM load off the resource-constrained VPS). The API deploys before any frontend. GitHub Actions' path filtering ensures a frontend-only change never triggers an API image rebuild. No `npm_config_*` env vars anywhere — pnpm v11 requires `pnpm_config_*`.

## Image build strategy — GitHub Actions builds, Coolify only pulls

**This project does not build Dockerfiles on the VPS.** Building 6 multi-stage images concurrently on a CX22/CX32 VPS competes with the very containers it's trying to serve, so the build step is offloaded to GitHub Actions. Coolify's Build Pack stays **Docker Compose** (see "Deployment architecture" below) — the change is that the compose file's per-service `build:` blocks are replaced with `image: ghcr.io/ru5ty-gate/<service>:main`, so Coolify's compose step pulls each image from GHCR instead of building it.

```
git push main
  → .github/workflows/continuous-integration.yml (GitHub-hosted runner)
      → path-filtered per service (dorny/paths-filter) — only rebuilds services whose files changed
      → docker/build-push-action builds each Dockerfile (context = repo root, same Dockerfiles as before)
      → pushes ghcr.io/ru5ty-gate/<service>:main and :sha-<short-sha>
  → Coolify (webhook or polling) sees the new :main image digest → pulls it → zero-downtime swap
```

The Dockerfiles themselves are unchanged and still authoritative — GitHub Actions builds from the exact same `apps/*/Dockerfile` files documented below, it just runs the build off-VPS instead of on it.

**Registry**: GHCR (`ghcr.io/ru5ty-gate/<service>`), auth via the workflow's built-in `GITHUB_TOKEN` (`packages: write` permission) — no separate registry account needed for push.

**Coolify side (one-time setup on the Docker Compose resource)**:
1. Build Pack stays **Docker Compose**, Compose file `docker-compose.yaml` — unchanged from before.
2. Add a GHCR pull credential (project-level Docker Registry, or on the resource): username = a GitHub username, password = a GitHub PAT with `read:packages` scope (needed because GHCR images default to private, inheriting the source repo's visibility).
3. Because the compose file's `image:` tag never changes (`:main`), enable Coolify's **"Force pull image"** / redeploy-on-webhook behavior (or run the deploy via the GHCR `package_published` webhook) — otherwise Coolify may see the same tag and skip pulling the new digest. `pull_policy: always` is set in `docker-compose.yaml` for exactly this reason.
4. Trigger: either GHCR's webhook on package push, or Coolify's own periodic image-check polling. A manual **Redeploy** from the Coolify UI always force-pulls.

Root `docker-compose.yaml` reflects this: every service uses `image: ghcr.io/ru5ty-gate/<service>:main` with `pull_policy: always` — there is no `build:` block anywhere in that file. `apps/*/Dockerfile` files still exist and are still the single source of truth for how each image is built; they're just invoked by `.github/workflows/continuous-integration.yml` instead of by Coolify.

**Rollback**: every build also pushes a `:sha-<short-sha>` tag. To roll back, change the Coolify application's image tag from `:main` to the last-known-good `:sha-...` and redeploy — no rebuild needed.

## VPS specification

| Resource | Minimum | Recommended |
|---|---|---|
| Provider | Hetzner Cloud | Hetzner Cloud |
| Instance | CX22 (2 vCPU, 4 GB) | CX32 (4 vCPU, 8 GB) |
| OS | Ubuntu 24.04 LTS | Ubuntu 24.04 LTS |
| Storage | 40 GB SSD | 80 GB SSD |
| Region | Closest to client | EU (Falkenstein) default |

Coolify installs and manages its own Traefik proxy, SSL (Let's Encrypt), and container orchestration on top of the bootstrapped server — nothing further is installed bare-metal.

## Deployment architecture — single Docker Compose stack

Coolify deploys this repo as **one Docker Compose resource** (Build Pack = Docker Compose). The root `docker-compose.yaml` declares every service as `image: ghcr.io/ru5ty-gate/<service>:main` — Coolify **pulls** each from GHCR, it does not build from the Dockerfiles (those are built by GitHub Actions; see "Image build strategy" above). Postgres and Redis are separate Coolify-managed resources.

```
GitHub Repo
└── .github/workflows/continuous-integration.yml builds & pushes to GHCR (not Coolify)
└── Coolify Resource: Docker Compose  →  docker-compose.yaml (image: only, no build:)
        ├── migrate        ghcr.io/ru5ty-gate/admin-api:main       (runs the migrate subcommand, exits)
        ├── api-gateway     ghcr.io/ru5ty-gate/api-gateway:main      EXPOSE 4000
        ├── admin-api       ghcr.io/ru5ty-gate/admin-api:main        EXPOSE 4001
        ├── customer-api    ghcr.io/ru5ty-gate/customer-api:main     EXPOSE 4002
        ├── schedule-api    ghcr.io/ru5ty-gate/schedule-api:main     EXPOSE 4003
        ├── customer-web    ghcr.io/ru5ty-gate/customer-web:main     EXPOSE 3000
        └── admin-web       ghcr.io/ru5ty-gate/admin-web:main        EXPOSE 80

Managed separately in Coolify:
        PostgreSQL resource  →  DATABASE_URL
        Redis resource       →  REDIS_URL
```

The `apps/*/Dockerfile` files (below) are still what GitHub Actions builds from — build context and `EXPOSE` values are unchanged, only *where* the build runs has moved.

Dockerfiles live **at each app root**, not in `devops/`. Build context is always the **monorepo root**.

No `config/deploy.*.yml` and no `.kamal/` directory — Coolify holds the per-application configuration (env vars, domain, resources) in its own database. The one exception is `.github/workflows/continuous-integration.yml`, which builds the images (see "Image build strategy" above) — that workflow is required in this project, unlike the vanilla Coolify-builds-on-VPS template. Do not alter the existing `devops/`, `common/`, `apps/`, or `turbo.json` structure to fit this.

## pnpm v11 requirements (frontend and mobile only)

The backend is a Cargo workspace and does not use pnpm. `pnpm-workspace.yaml` lists only the JavaScript apps:

```yaml
packages:
    - "apps/frontend/*"
    - "apps/mobile/*"
    # apps/cms is excluded — managed independently with npm

# Required for pnpm v11 in Docker — prevents a network hit on every
# `pnpm run` during the build:
verifyDepsBeforeRun: false

allowBuilds:
    core-js: true
    cypress: true
    esbuild: true
    sharp: true
    unrs-resolver: true
```

Add new native packages here as they surface in `ERR_PNPM_IGNORED_BUILDS` errors. Only registry and auth settings belong in `.npmrc` — `verify-deps-before-run`, `hoist-pattern`, `node-linker`, `save-exact` are silently ignored there in v11. `overrides` and `patchedDependencies` no longer live under `package.json#pnpm` — declare them in `pnpm-workspace.yaml` instead.

## Dockerfiles

Every Dockerfile lives **at its app root**. Build context is always the **monorepo root** (`/`). The `EXPOSE` port must match the `PORT` env var set in `docker-compose.yaml`.

| Dockerfile | Cargo package / binary, or pnpm filter | `EXPOSE` |
|---|---|---|
| `apps/backend/api-gateway/Dockerfile` | `ru5ty-gate-api-gateway` / `api-gateway` | `4000` |
| `apps/backend/admin-api/Dockerfile` | `ru5ty-gate-admin-api` / `admin-api` | `4001` |
| `apps/backend/customer-api/Dockerfile` | `ru5ty-gate-customer-api` / `customer-api` | `4002` |
| `apps/backend/schedule-api/Dockerfile` | `ru5ty-gate-schedule-api` / `schedule-api` | `4003` |
| `apps/frontend/admin-web/Dockerfile` | `admin-web` | `80` |
| `apps/frontend/customer-web/Dockerfile` | `customer-web` | `3000` |

**Do not copy port values from other sources.** The 3000–3003 range seen in some older references is wrong.

### Backend API (canonical pattern — shown for `customer-api`)

```dockerfile
FROM rust:1.97-slim-bookworm AS chef
RUN apt-get update \
    && apt-get install -y --no-install-recommends build-essential cmake pkg-config \
    && rm -rf /var/lib/apt/lists/*
RUN cargo install cargo-chef --locked
WORKDIR /app

FROM chef AS planner
COPY Cargo.toml Cargo.lock ./
COPY common/ ./common/
COPY apps/backend/ ./apps/backend/
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
ENV CARGO_INCREMENTAL=0
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --locked --recipe-path recipe.json -p ru5ty-gate-customer-api
COPY Cargo.toml Cargo.lock ./
COPY common/ ./common/
COPY apps/backend/ ./apps/backend/
RUN cargo build --release --locked -p ru5ty-gate-customer-api --bin customer-api

FROM gcr.io/distroless/cc-debian12:nonroot AS runner
ENV APP_ENV=production
COPY --from=builder /app/target/release/customer-api /usr/local/bin/customer-api
USER nonroot
EXPOSE 4002
HEALTHCHECK --interval=15s --timeout=5s --retries=3 \
  CMD ["/usr/local/bin/customer-api", "healthcheck"]
ENTRYPOINT ["/usr/local/bin/customer-api"]
```

The dependency layer is cached by `cargo-chef`, so code-only changes rebuild just the service crate. Distroless has no shell, `curl` or `wget`, which is why the binary itself implements the `healthcheck` subcommand. The committed Dockerfiles are the source of truth; copy `customer-api`'s and change the package name, binary name, `EXPOSE` and `HEALTHCHECK` for another service.

### Frontend Dockerfiles

`admin-web` and `customer-web` Dockerfiles live at `apps/frontend/admin-web/Dockerfile` and `apps/frontend/customer-web/Dockerfile`. Frontend package names are **unscoped** (`admin-web`, `customer-web`):

- `admin-web` — Vite SPA built to `dist/`, served by `nginx:alpine` on port `80`. Build args: `VITE_ADMIN_API_BASE_URL`, `VITE_ADMIN_APP_NAME`.
- `customer-web` — Next.js with `output: "standalone"` (set in `next.config.mjs`); the runner copies `.next/standalone`, `.next/static`, and `public`, runs `node apps/frontend/customer-web/server.js` on `:3000` (`PORT`/`HOSTNAME` env). Build args: `NEXT_PUBLIC_CUSTOMER_API_BASE_URL`, `NEXT_PUBLIC_CUSTOMER_APP_NAME`.
- The `NEXT_PUBLIC_*` / `VITE_*` args are declared before the build step and must be set as **Build Variables** in Coolify so they are baked into the bundle.

## Coolify applications

Coolify deploys this repo as **one Docker Compose resource** (see the architecture section above), not one Application per service. Its Build Pack is **Docker Compose**, Base Directory `/`, Compose file `docker-compose.yaml` — and because that compose file now declares `image:` (GHCR) instead of `build:` for every service, Coolify pulls each image rather than building it. There is no per-service "Build Pack: Dockerfile" application to configure; the table below documents each service's port/domain/health-check for reference, not a separate Coolify resource.

| Service | GHCR image | Port | Domain (FQDN) | Health check |
|---|---|---|---|---|
| api-gateway | `ghcr.io/ru5ty-gate/api-gateway` | `4000` | `https://api.<domain>` | `/health/live` (`4000`) |
| customer-api | `ghcr.io/ru5ty-gate/customer-api` | `4002` | internal | `/api/v1/ping` (`4002`) |
| admin-api | `ghcr.io/ru5ty-gate/admin-api` | `4001` | internal | `/api/v1/ping` (`4001`) |
| schedule-api | `ghcr.io/ru5ty-gate/schedule-api` | `4003` | internal | `/api/v1/ping` (`4003`) |
| admin-web | `ghcr.io/ru5ty-gate/admin-web` | `80` | `https://admin.<domain>` | `/` (`80`) |
| customer-web | `ghcr.io/ru5ty-gate/customer-web` | `3000` | `https://<domain>` | `/` (`3000`) |

Coolify still reads the exposed port from each Dockerfile's `EXPOSE` (baked into the image at GitHub Actions build time) and routes the configured domain to it through its managed Traefik proxy — SSL is provisioned automatically via Let's Encrypt once DNS resolves to the VPS. Add the GHCR pull credential (see "Image build strategy" above) at the Docker Compose resource level so every service image can be pulled.

## Managed resources (Postgres + Redis)

Create these as Coolify **Resources** in the same project, not as external managed services:

- **PostgreSQL** — Coolify's PostgreSQL resource (use the TimescaleDB image variant, `timescale/timescaledb:latest-pg16`, if hypertables are needed). Coolify generates the credentials and an internal connection URL; persistent storage is managed by Coolify volumes.
- **Redis** — Coolify's Redis resource with a password set; persistence (`appendonly yes`) enabled in the resource's custom start command if durable queues are required.

Reference these from the API application by their **internal connection URL** (Coolify exposes it on the resource page) — internal networking means the database is never published to the public internet. Set `DATABASE_URL` and `REDIS_URL` on the API application to those internal URLs.

## Environment variables

Set these on each application under **Environment Variables** in Coolify. Mark secrets as secret; values consumed only during `docker build` (e.g. `NEXT_PUBLIC_*`) must be flagged **Build Variable** so they are baked into the image.

### API application

| Variable | Notes |
|---|---|
| `APP_ENV` | `production` |
| `PORT` | the service port (4000–4003) |
| `DATABASE_URL` | internal Postgres URL from the Coolify resource |
| `REDIS_URL` | internal Redis URL from the Coolify resource |
| `JWT_SECRET`, `JWT_REFRESH_SECRET` | secret |
| `CORS_ORIGIN` | the customer-web/admin origins |
| `TWO_FACTOR_ENCRYPTION_KEY` | secret, 64 hex characters (admin-api) |
| `SCHEDULE_API_KEY` | secret, at least 32 characters (schedule-api) |
| `MAILTRAP_API_KEY`, `MAILTRAP_FROM`, `MAILTRAP_FROM_NAME` | secret |
| `S3_BUCKET`, `S3_REGION`, `S3_ACCESS_KEY_ID`, `S3_SECRET_ACCESS_KEY`, `S3_ENDPOINT` | secret (if using S3 or R2 storage) |

### Admin application

| Variable | Notes |
|---|---|
| `ADMIN_BOOTSTRAP_ENABLED` | `false` normally; `true` only for the one-time first-admin call |

### Customer Web application

| Variable | Notes |
|---|---|
| `NODE_ENV` | `production` |
| `PORT` | `3000` |
| `NEXT_PUBLIC_API_URL` | **Build Variable** — baked at build time |

Never commit these values anywhere — Coolify stores and injects them at deploy time.

## Common package env vars (who reads what)

| Package | Environment Variables |
|---|---|
| `common/database` | `DATABASE_URL` |
| `common/cache` | `REDIS_URL` |
| `common/queue` | `REDIS_URL` |
| `common/email` | `MAILTRAP_API_KEY`, `MAILTRAP_FROM`, `MAILTRAP_FROM_NAME` |
| `common/storage` | `S3_BUCKET`, `S3_REGION`, `S3_ACCESS_KEY_ID`, `S3_SECRET_ACCESS_KEY`, `S3_ENDPOINT`, `S3_PUBLIC_URL`, or the `R2_*` equivalents with `R2_ACCOUNT_ID` / `R2_ENDPOINT` (Azure Blob storage is not supported by the Rust crate) |
| `common/logging` | `APP_ENV`, `LOG_LEVEL` |
| `common/metrics` | none — exposes `/metrics` on the service port |
| `common/config` | `EnvReader` — the single entry point for env in every service |

## Full environment variable contract

Source of truth is `devops/.env.example`. All vars are set in the Coolify Docker Compose resource's Environment Variables panel. Mark secrets as **Secret**; mark `NEXT_PUBLIC_*` and `VITE_*` as **Build Variables**.

| Variable | Service(s) |
|---|---|
| `APP_ENV` | all backend (`production`) |
| `PORT` | all backend |
| `DATABASE_URL` | admin-api, customer-api, schedule-api, migrate |
| `REDIS_URL` | admin-api, customer-api, schedule-api |
| `REDIS_TLS_REJECT_UNAUTHORIZED` | admin-api, customer-api, schedule-api (`false` for Coolify-managed Redis) |
| `CORS_ORIGIN` | all backend |
| `JWT_SECRET` | admin-api, customer-api |
| `JWT_REFRESH_SECRET` | admin-api, customer-api |
| `TWO_FACTOR_ENCRYPTION_KEY` | admin-api (64 hex characters) |
| `ADMIN_BOOTSTRAP_ENABLED` | admin-api |
| `PASSWORD_RESET_EXPIRATION_MINUTES` | admin-api |
| `SCHEDULE_API_KEY` | schedule-api |
| `SCHEDULE_WEBHOOK_INTERVAL_SECONDS` | schedule-api |
| `MAILTRAP_API_KEY`, `MAILTRAP_FROM`, `MAILTRAP_FROM_NAME` | admin-api, customer-api |
| `ADMIN_WEB_URL` | admin-api |
| `CUSTOMER_WEB_URL` | customer-api |
| `ADMIN_API_URL`, `CUSTOMER_API_URL`, `SCHEDULER_API_URL` | api-gateway |
| `RATE_LIMIT_MAX` | api-gateway |
| `GRAPHQL_ENABLED`, `GRAPHQL_PATH` | api-gateway (optional; playground and introspection are forced off in production) |
| `SENTRY_DSN`, `SENTRY_RELEASE`, `SENTRY_TRACES_SAMPLE_RATE` | all backend (optional) |
| `SMSPORTAL_CLIENT_ID`, `SMSPORTAL_API_SECRET`, `SMSPORTAL_ENABLED`, `SMSPORTAL_SENDER_ID` | services that send SMS (optional) |
| `NEXT_PUBLIC_CUSTOMER_API_BASE_URL` | customer-web (build arg) |
| `VITE_ADMIN_API_BASE_URL` | admin-web (build arg) |

Never pass secrets through committed files. Never hardcode values in Dockerfiles or docker-compose.yaml.

## CI/CD — GitHub Actions builds, Coolify redeploys on new image

Two separate triggers now cooperate, both firing off the same push to `main`:

1. **`.github/workflows/continuous-integration.yml`** — path-filtered per service, builds only the images whose files changed, pushes `:main` and `:sha-<short-sha>` to GHCR.
2. **Coolify** — configured on the single Docker Compose resource with Auto Deploy on. It redeploys when it detects the `:main` tag's digest changed, either via a webhook from GHCR (`package_published` event, if wired up) or Coolify's periodic image-check polling. A manual **Redeploy** from the Coolify UI always force-pulls immediately, so that's the reliable fallback right after a GitHub Actions run finishes.

Flow: push to `main` → GitHub Actions builds only the changed services (path filter) → pushes new `:main` image(s) to GHCR → Coolify pulls the updated image(s) (`pull_policy: always`) → zero-downtime container swap → the one-shot `migrate` service runs the pending migrations before dependent services start (`depends_on: condition: service_completed_successfully`).

Deploy ordering (API before frontends) is enforced by the compose file's `depends_on`/`condition: service_healthy` graph, not by watch paths — there's only one Coolify resource, so there's nothing to order across separate applications.

## Migrations — the `migrate` compose job

Migrations run through the one-shot `migrate` service in the root `docker-compose.yaml`. It reuses the `admin-api` image, runs the `migrate` subcommand (which applies the migrations embedded in the binary from `common/database/migrations`) and exits:

```yaml
migrate:
    image: ghcr.io/ru5ty-gate/admin-api:main
    pull_policy: always
    command: ["migrate"]
    environment:
        DATABASE_URL: ${DATABASE_URL}
    restart: "no"
```

`admin-api`, `customer-api` and `schedule-api` declare `depends_on: migrate: condition: service_completed_successfully`, so no API starts against an unmigrated schema, including on a fresh empty database. Migrations are idempotent — already-applied files are skipped — so they are safe on every deploy. Do not also set a Coolify Post-deployment Command; the compose job is the single mechanism. `devops/scripts/migrate-deploy.sh` is a thin wrapper that runs the same subcommand for manual use inside the container.

Reference data (roles, user statuses, lookups) is created by a migration, so a fresh database is usable immediately. The first administrator is created with the one-time `POST /auth/bootstrap-admin` call (set `ADMIN_BOOTSTRAP_ENABLED=true` for that call, then back to `false`).

## First deploy checklist (once per project)

1. VPS bootstrapped and Coolify installed — see `vps-bootstrap`.
2. Ensure `.github/workflows/continuous-integration.yml` exists and has run at least once on `main` (all 6 images pushed to GHCR) — Coolify's first pull needs the images to already exist.
3. In Coolify: create a **Project**, add a Docker Registry credential for GHCR (username = GitHub username, password = a GitHub PAT with `read:packages`) so private GHCR images can be pulled.
4. Create **PostgreSQL** and **Redis** as Coolify-managed resources. Copy their internal connection strings.
5. Create a **Docker Compose** resource: Build Pack = Docker Compose, Base Directory = `/`, Compose file = `docker-compose.yaml`. Since every service uses `image:` (not `build:`), Coolify pulls from GHCR using the credential from step 3 — it does not build anything.
6. In the resource's **Environment Variables** panel, add every var from `devops/.env.example` with real values. Mark secrets as **Secret**; mark `NEXT_PUBLIC_*` and `VITE_*` as **Build Variables** (these are baked in by GitHub Actions, not by Coolify — set them as repo/environment **Variables** in GitHub Actions too, see the `continuous-integration.yml` matrix `build-args`).
7. Set domains: `api.<domain>` on api-gateway (port 4000), `admin.<domain>` on admin-web (port 80), `<domain>` on customer-web (port 3000).
8. Point DNS at the VPS, then trigger the first deploy from the Coolify UI.

After this, every push to `main` triggers GitHub Actions to rebuild the changed image(s); Coolify picks up the new `:main` digest and redeploys (see "CI/CD" above). The `migrate` service in docker-compose handles schema migrations before any API container starts, via `depends_on: condition: service_completed_successfully`.

## Per-project substitutions

| Placeholder | Replace with |
|---|---|
| `<project>` | e.g. `my-project` |
| `<domain>` | e.g. `myproject.co.za` |

Each project gets its own Coolify project and its own resources — never shared across projects.

## DNS and Cloudflare

One VPS IP per project. Coolify's Traefik proxy listens on 80/443 and routes by `Host` header — all DNS records point to the same VPS IP, and each application's configured FQDN is matched automatically.

```
DNS (Cloudflare)        Traefik (VPS :443)     Container
<domain>            ──► customer-web           :3000 (Next.js)
admin.<domain>      ──► admin-web              :80   (nginx)
api.<domain>        ──► api-gateway            :4000
```

`admin-api`, `customer-api`, and `schedule-api` are internal — they communicate inside the Docker network and are not exposed to the internet.

Use a wildcard A record, not individual records per subdomain:

```
A   <domain>      <VPS_IP>   Proxied
A   *.<domain>    <VPS_IP>   Proxied
```

Cloudflare SSL mode must be Full (strict) — Flexible breaks Let's Encrypt provisioning (Cloudflare to VPS over plain HTTP); Full (non-strict) works but uses a self-signed cert; only Full (strict) is correct.

Adding a new subdomain needs no DNS change (wildcard covers it) — just a new service block in `docker-compose.yaml` with its own GHCR image and FQDN configured on the Docker Compose resource.

## What Claude Code must not do

Never trigger a Coolify deploy — the developer does this from the Coolify UI or via git push. Never run migrations manually — the `migrate` compose job handles it. Never write a migration script inside an app folder — migrations are SQL files in `common/database/migrations` applied by the `migrate` subcommand. Never run `git` commands. Never hardcode secret values — they live only in Coolify's Environment Variables (and GitHub Actions' repo/environment Variables for `NEXT_PUBLIC_*`/`VITE_*` build args). Never modify `turbo.json`, `pnpm-workspace.yaml` package globs, or `common/*` structure to fit deployment needs. Never add `npm_config_*` env vars — use `pnpm_config_*`. Never write pnpm behavior settings to `.npmrc`. Never add `onlyBuiltDependencies`/`neverBuiltDependencies` — use `allowBuilds`. Never skip `ENV CI=true` or the `pnpm-workspace.yaml` copy step in a frontend Dockerfile, and never add a shell, `curl` or `wget` to a backend image for healthchecks. Never add a `build:` block back to `docker-compose.yaml` — it must stay `image:`-only so Coolify never builds on the VPS. Never create individual DNS A records per subdomain when the wildcard covers it. Never set Cloudflare SSL mode to anything but Full (strict).
