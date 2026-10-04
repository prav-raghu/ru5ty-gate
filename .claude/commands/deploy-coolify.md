---
description: Set up Coolify deployment for this project — per-app Dockerfiles, the root docker-compose stack, and the Coolify configuration checklist
argument-hint: <project name and domain, e.g. "burger-shop, burgershop.co.za">
---

Use the deployment-coolify subagent to set up the full Coolify deployment for: $ARGUMENTS

1. Each app has its own Dockerfile **inside its app folder**: `apps/backend/<service>/Dockerfile` for the four APIs and `apps/frontend/<app>/Dockerfile` for the two web apps. Create any that are missing using the templates below.
2. The root `docker-compose.yaml` declares every service as one stack. Coolify deploys the whole monorepo as a **single Docker Compose resource** — keep it that way.
3. Verify `pnpm-workspace.yaml` (frontend and mobile apps only) has a complete `allowBuilds` list for the frontends, and that the Rust backend images build from the committed Dockerfiles (`cargo-chef` + distroless).
4. Output the Coolify Compose resource configuration, the env vars / Build Variables each service needs, the managed Postgres/Redis setup, and the DNS/Cloudflare records.

Assumes the target VPS has already been bootstrapped and Coolify installed — if not, run vps-bootstrap first.

# Coolify Deployment Guide

Reference for deploying this monorepo to a self-hosted Coolify instance on a
Hetzner VPS. This is the canonical, only sanctioned deployment path. The VPS
must already be bootstrapped and have Coolify installed (see `vps-bootstrap`).

This reference backs the `/deploy-coolify` slash command and mirrors the
`deployment-coolify` subagent. If they ever disagree, **the committed
`docker-compose.yaml` and the per-app Dockerfiles are the source of truth.**

> **Images are built by GitHub Actions, not Coolify.** The committed
> `docker-compose.yaml` uses `image: ghcr.io/ru5ty-gate/<service>:main`
> with `pull_policy: always` and no `build:` key anywhere — images are built by
> `.github/workflows/continuous-integration.yml` and pushed to GHCR; Coolify only
> pulls. See the `deployment-coolify` subagent's "Image build strategy" section
> for the full flow. Do not add a `build:` block back to `docker-compose.yaml`.
>
> **Template note:** portable across projects forked from the monorepo template.
> When forking, replace `ru5ty-gate` (and `ru5ty_gate`
> in Rust code) with the new slug everywhere; the structure is identical.

---

## Architecture — single Compose stack

Coolify deploys this repo as **one Docker Compose resource** (build pack =
Docker Compose), not as separate Applications. The root `docker-compose.yaml`
defines every service; Coolify pulls the prebuilt GHCR images (built from each
app's own Dockerfile by GitHub Actions) and wires them onto one network behind
its Traefik proxy.

```text
GitHub Repo (ru5ty-gate/<project>)
└── Coolify Resource: Docker Compose  →  docker-compose.yaml
        ├── admin-api      apps/backend/admin-api/Dockerfile        :4001
        ├── customer-api   apps/backend/customer-api/Dockerfile     :4002
        ├── schedule-api   apps/backend/schedule-api/Dockerfile     :4003
        ├── api-gateway    apps/backend/api-gateway/Dockerfile      :4000
        ├── customer-web   apps/frontend/customer-web/Dockerfile    :3000 (Next.js)
        └── admin-web      apps/frontend/admin-web/Dockerfile       :80   (nginx, Traefik-routed)
```

Postgres and Redis are provisioned separately via Coolify's managed
Database/Service resources and referenced by their internal connection URL —
not declared in this compose file.

---

## Ports & package scopes (exact names matter for `--filter`)

| App | Cargo package / pnpm filter | Port |
|---|---|---|
| `api-gateway` | `ru5ty-gate-api-gateway` | 4000 |
| `admin-api` | `ru5ty-gate-admin-api` | 4001 |
| `customer-api` | `ru5ty-gate-customer-api` | 4002 |
| `schedule-api` | `ru5ty-gate-schedule-api` | 4003 |
| `customer-web` | `customer-web` (unscoped) | 3000 (Next.js) |
| `admin-web` | `admin-web` (unscoped) | 80 (nginx) |

---

## Backend Dockerfile template

Every backend service uses the **monorepo root as build context** so `Cargo.toml`,
`Cargo.lock` and `common/` are present. The build is a four-stage `cargo-chef`
build that ends in a distroless runtime image; the dependency layer is cached so
code-only changes rebuild just the service crate.

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
RUN cargo chef cook --release --locked --recipe-path recipe.json -p ru5ty-gate-<service>
COPY Cargo.toml Cargo.lock ./
COPY common/ ./common/
COPY apps/backend/ ./apps/backend/
RUN cargo build --release --locked -p ru5ty-gate-<service> --bin <service>

FROM gcr.io/distroless/cc-debian12:nonroot AS runner
ENV APP_ENV=production
COPY --from=builder /app/target/release/<service> /usr/local/bin/<service>
USER nonroot
EXPOSE <port>
HEALTHCHECK --interval=15s --timeout=5s --retries=3 \
  CMD ["/usr/local/bin/<service>", "healthcheck"]
ENTRYPOINT ["/usr/local/bin/<service>"]
```

Distroless has no shell, `curl` or `wget`, so the binary implements a
`healthcheck` subcommand (plain TCP `GET` against the service's ping route,
exit 0 or 1). Substitute `<service>` (e.g. `admin-api`) and `<port>` (4000–4003).

---

## Frontend Dockerfile template — React SPA (admin-web)

Vite SPA built to `dist/`, served by `nginx:alpine` on `:80` with an inline
SPA-history-fallback config so client-side routes resolve to `index.html`.
Frontend package name is **unscoped** (`admin-web`). `VITE_*` vars are `ARG`s
**before** the build and must be set as Build Variables in Coolify.

```dockerfile
FROM node:22-alpine AS base
ENV CI=true
ENV PNPM_HOME="/pnpm"
ENV PATH="$PNPM_HOME:$PATH"
RUN corepack enable && corepack prepare pnpm@11.5.2 --activate

FROM base AS builder
WORKDIR /app
COPY pnpm-workspace.yaml pnpm-lock.yaml package.json turbo.json ./
COPY common/ ./common/
COPY apps/frontend/admin-web/ ./apps/frontend/admin-web/
RUN pnpm install --frozen-lockfile

ARG VITE_ADMIN_API_BASE_URL
ARG VITE_ADMIN_APP_NAME=Admin
ENV VITE_ADMIN_API_BASE_URL=$VITE_ADMIN_API_BASE_URL
ENV VITE_ADMIN_APP_NAME=$VITE_ADMIN_APP_NAME

RUN pnpm --filter admin-web build

FROM nginx:alpine AS runner
COPY --from=builder /app/apps/frontend/admin-web/dist /usr/share/nginx/html
RUN printf 'server {\n\
    listen 80;\n\
    root /usr/share/nginx/html;\n\
    index index.html;\n\
    add_header X-Frame-Options "DENY" always;\n\
    add_header X-Content-Type-Options "nosniff" always;\n\
    add_header Referrer-Policy "strict-origin-when-cross-origin" always;\n\
    location / {\n\
        try_files $uri $uri/ /index.html;\n\
        expires 1h;\n\
        add_header Cache-Control "public, must-revalidate, proxy-revalidate";\n\
    }\n\
    location ~* \.(js|css|png|jpg|jpeg|gif|ico|svg|woff|woff2|ttf|eot)$ {\n\
        expires 1y;\n\
        add_header Cache-Control "public, immutable";\n\
        access_log off;\n\
    }\n\
    error_page 404 /index.html;\n\
}\n' > /etc/nginx/conf.d/default.conf
EXPOSE 80
CMD ["nginx", "-g", "daemon off;"]
```

---

## Frontend Dockerfile template — Next.js (customer-web)

Next.js with `output: "standalone"` and `outputFileTracingRoot` pointing at the
repo root in `next.config.mjs`. The runner copies `.next/standalone`,
`.next/static`, and `public`, and runs `server.js`. Declare every
`NEXT_PUBLIC_*` var as an `ARG`+`ENV` **before** the build (abridged below) and
set them as Build Variables in Coolify, or they will not be embedded.

```dockerfile
FROM node:22-alpine AS base
ENV CI=true
ENV PNPM_HOME="/pnpm"
ENV PATH="$PNPM_HOME:$PATH"
RUN corepack enable && corepack prepare pnpm@11.5.2 --activate

FROM base AS builder
WORKDIR /app
COPY pnpm-workspace.yaml pnpm-lock.yaml package.json turbo.json ./
COPY common/ ./common/
COPY apps/frontend/customer-web/ ./apps/frontend/customer-web/
RUN pnpm install --frozen-lockfile

ARG NEXT_PUBLIC_CUSTOMER_API_BASE_URL
ARG NEXT_PUBLIC_CUSTOMER_SITE_URL
# ... declare every NEXT_PUBLIC_* the app reads ...
ENV NEXT_PUBLIC_CUSTOMER_API_BASE_URL=$NEXT_PUBLIC_CUSTOMER_API_BASE_URL
ENV NEXT_PUBLIC_CUSTOMER_SITE_URL=$NEXT_PUBLIC_CUSTOMER_SITE_URL
# ... matching ENV lines ...

RUN pnpm --filter @ru5ty-gate/customer-web build

FROM node:22-alpine AS runner
WORKDIR /app
ENV NODE_ENV=production
RUN addgroup --system --gid 1001 nodejs && adduser --system --uid 1001 nextjs
COPY --from=builder /app/apps/frontend/customer-web/public ./apps/frontend/customer-web/public
COPY --from=builder --chown=nextjs:nodejs /app/apps/frontend/customer-web/.next/standalone ./
COPY --from=builder --chown=nextjs:nodejs /app/apps/frontend/customer-web/.next/static ./apps/frontend/customer-web/.next/static
USER nextjs
EXPOSE 3000
ENV PORT=3000
ENV HOSTNAME="0.0.0.0"
CMD ["node", "apps/frontend/customer-web/server.js"]
```

> Keep `EXPOSE`/`ENV PORT` consistent with the port the compose service sets.
> Next.js reads `PORT` at runtime; the compose `environment.PORT` wins if they
> differ, but mismatched values are a footgun — align them on `3000`.

---

## docker-compose.yaml

The single stack Coolify builds. Backends expose their port internally;
`admin-web` carries the Traefik label and is the public entrypoint. APIs declare
a **healthcheck** hitting their real liveness route — the path differs by
service because the health routes mount under each service's route prefix:

- `admin-api`, `customer-api` and `schedule-api` mount health under `api/v1/` → **`/api/v1/ping`**
- `api-gateway` has a dedicated self-liveness route → **`/health/live`** (returns
  200 whenever the process is up; distinct from `/health/services`, which
  aggregates downstream status and is a *readiness*, not liveness, probe)

The compose healthcheck for every backend is the exec form
`["CMD", "/usr/local/bin/<service>", "healthcheck"]`, which calls those paths from inside
the container.

`/ping` is a pure 200 (no DB) and is the correct liveness target. Probing a non-existent path is the classic cause of a Coolify "Unhealthy"
status. For the gateway, do **not** use `/health/services` as the container
healthcheck — it returns 503 when any downstream is briefly down and would make
the gateway flap unhealthy in a cascade.

Inter-service URLs use **compose service names**, not `localhost` — inside the
stack the gateway reaches the APIs at `http://admin-api:4001`,
`http://customer-api:4002`, `http://schedule-api:4003`.

**Read the committed `docker-compose.yaml` at the repo root directly — it is the single source of truth for this stack, not an example here.** It declares every service `image:`-only (GHCR, `pull_policy: always`), with no `build:` key anywhere — see `rules/docker.md` for why, and `.github/workflows/continuous-integration.yml` for how each image actually gets built and pushed. Do not reconstruct the compose file from a written example; if this doc and the committed file ever disagree, the committed file wins.

`schedule-api` and `api-gateway` are **optional** — a project only deploys them if it actually uses the scheduler / gateway; when unused, comment out their service blocks (and remove them from any `depends_on` lists) rather than deleting them.

Both frontends are public via their own Coolify domain (set in the Coolify UI, which auto-injects the Traefik routing labels — the compose file itself declares no `labels:`); the APIs are reached internally on `app-network` (and `coolify`, for managed Postgres/Redis). If a frontend returns Traefik's `504 Gateway Timeout` while its own container logs look healthy, check for a missing Domain in the Coolify UI before suspecting the app — a partial/manual Traefik label collides with Coolify's auto-injection and produces a router with no working `Host()` match.

---

## Coolify resource settings

Create **one** resource: **New Resource → Docker Compose**, connect the GitHub
repo, branch `main`, Auto Deploy on.

| Field | Value |
|---|---|
| **Build Pack** | `Docker Compose` |
| **Base Directory** | `/` |
| **Compose File** | `docker-compose.yaml` |
| **Branch** | `main`, Auto Deploy on |

The compose file references prebuilt GHCR images (`image:` only); GitHub Actions
builds them from each app's Dockerfile with the repo root as context. Do not set a
subfolder base directory.

---

## Environment Variables

Set all env vars on the Coolify Compose resource. The compose file interpolates
`${VAR}` from Coolify's environment. Mark secrets as secret. `NEXT_PUBLIC_*` and
`VITE_*` are consumed at **build** time (they appear under `build.args`), so they
must be present when Coolify builds, not only at runtime.

`DATABASE_URL` and `REDIS_URL` point at the managed Postgres/Redis internal URLs
(below). Never commit secret values.

---

## Managed Postgres & Redis

Provision via Coolify's **New Resource → Database** — not in compose. They live
on Coolify's own **`coolify`** Docker network and are addressed by an
internal hostname (a random ID like `a6fkd1l5liyzwg6psnwjmdwt`).

| Service | Coolify Resource |
|---|---|
| PostgreSQL 16 | Database → PostgreSQL |
| Redis 7 | Database → Redis (set a password) |

Set `DATABASE_URL` / `REDIS_URL` on the Compose resource to those internal URLs.

### The services must join the `coolify` network

A self-contained compose `app-network` **cannot** resolve the managed
resources' hostnames — the symptom is `getaddrinfo EAI_AGAIN <random-id>` (DNS
failure) on Redis/Postgres connect. The fix: declare `coolify` as an **external**
network and attach the API services to it (in addition to `app-network`).

```yaml
services:
    admin-api:
        # ...
        networks:
            - app-network
            - coolify   # reach managed Redis / Postgres

networks:
    app-network:
        driver: bridge
    coolify:
        external: true
```

Declare `coolify` explicitly only for the services that talk to managed
resources (the APIs). The frontends are listed on `app-network` only **in this
file**, but Coolify attaches them to `coolify` automatically when you set their
Domain in the UI (see "Public routing is set in the Coolify UI, not in compose"
above) — so do not add it to them by hand.

### Managed Redis uses TLS with a self-signed cert

Coolify's managed Redis is reached over `rediss://` and presents a self-signed
certificate. Once the network is right, the next error is `self-signed
certificate in certificate chain`. The cache client accepts the managed cert by
default (`rejectUnauthorized: false` for `rediss://`); set
`REDIS_TLS_REJECT_UNAUTHORIZED=true` on the Compose resource only if you supply
a CA-signed Redis cert. See `documentation/common-package-env-config.md`.

> **The Redis error walks down the stack** — each error means the previous layer
> is fixed: `ECONNREFUSED 127.0.0.1` (URL not read) → `EAI_AGAIN <id>` (wrong
> network) → `self-signed certificate` (TLS) → `Redis connected successfully`.

---

## Migrations — the `migrate` compose job

Migrations run via the one-shot **`migrate`** service in the root
`docker-compose.yaml`. It reuses the admin-api image (the migrations are embedded
in the binary), runs the `migrate` subcommand, then exits. `admin-api`,
`customer-api` and `schedule-api` gate on it with
`condition: service_completed_successfully`, so the schema is always current
before any traffic, including on a fresh empty database.

```yaml
migrate:
    image: ghcr.io/ru5ty-gate/admin-api:main
    pull_policy: always
    command: ["migrate"]
    environment:
        DATABASE_URL: ${DATABASE_URL}
    restart: "no"
```

> **Do NOT also set a Coolify Post-deployment Command for migrations.** The compose
> `migrate` job is the single source of truth; a post-deploy command would double-run
> it.

Applied migrations are tracked in the `_sqlx_migrations` table, and the runner is
idempotent — it applies only pending migrations and no-ops when none are pending,
so it is safe on every deploy. Never run migrations manually once the `migrate`
job is in place.

### Seeding

Reference data (roles, user statuses, lookups) is created by a migration, so there is
nothing to seed by hand. The first administrator is created through the one-time
`POST /api/v1/auth/bootstrap-admin` call on `admin-api` (set
`ADMIN_BOOTSTRAP_ENABLED=true`, make the call, then set it back to `false`). The
`seed` binary in `common/database` is for development data only.

---

## DNS and Cloudflare

One VPS IP per project. Coolify's Traefik listens on 80/443 and routes by `Host`
header. Use a wildcard A record:

```text
A   <domain>      <VPS_IP>   Proxied
A   *.<domain>    <VPS_IP>   Proxied
```

Cloudflare SSL mode must be **Full (strict)** — Flexible breaks Let's Encrypt.

---

## Agent notes

- Dockerfiles live **inside each app folder** (`apps/backend/<service>/Dockerfile`,
  `apps/frontend/<app>/Dockerfile`) — not a centralized `devops/docker/`.
- Build context is always the monorepo root (set in the GitHub Actions docker-build matrix).
- Use the exact Cargo package name (`ru5ty-gate-<service>`) for backends and
  the unscoped pnpm filter (`admin-web`, `customer-web`) for frontends.
- Keep the single `docker-compose.yaml` — do not split into per-app Coolify
  Applications.
- Do **not** hand-write Traefik labels or attach the `coolify` network to the
  frontends — Coolify injects both when a Domain is set in the UI. A partial
  manual label (port without a `Host()` router) causes a 504 on an otherwise
  healthy nginx container.
- Backend healthchecks use the binary's `healthcheck` subcommand; it must target the real route (`/api/v1/ping`, or `/health/live` on the gateway).
- Never trigger a Coolify deploy, run migrations manually, run `git`, hardcode
  secrets, or modify `turbo.json` / `pnpm-workspace.yaml` globs / `common/*`
  structure to fit deployment.
