---
paths:
  - "**/Dockerfile"
  - "docker-compose*.yaml"
  - "docker-compose*.yml"
---

# Docker Rules

You are working on Docker configuration for this monorepo.

## Monorepo structure

```
/ (monorepo root)
├── docker-compose.yaml          ← single Coolify production stack (all deployable services)
├── Cargo.toml / Cargo.lock      ← Rust workspace for every backend service and common crate
├── apps/
│   ├── backend/
│   │   ├── api-gateway/   └── Dockerfile
│   │   ├── admin-api/     └── Dockerfile
│   │   ├── customer-api/  └── Dockerfile
│   │   └── schedule-api/  └── Dockerfile
│   └── frontend/
│       ├── admin-web/     └── Dockerfile   (Vite SPA → nginx)
│       └── customer-web/  └── Dockerfile   (Next.js standalone)
├── common/                      ← shared crates only, never a Dockerfile
└── devops/                      ← local dev infra + scripts only, never deployed
    └── docker-compose.dev.yml   ← local dev only (Postgres, Redis, Adminer)
```

One `Dockerfile` per deployable app, located at the app root. `devops/` contains local infrastructure and scripts only — never a Dockerfile. `common/*` are shared libraries — never get their own Dockerfile. Do not create `docker-compose.qa.yml` or `docker-compose.prod.yml` — Coolify environments handle this.

## Build context is always the monorepo root

Every Dockerfile uses the repo root as its build context so it can access `Cargo.toml`, `Cargo.lock` and `common/`. The Dockerfile location is inside the app, but the context is always `/`.

## Backend Dockerfile pattern (Rust, required)

| Stage | Purpose |
|---|---|
| `chef` | `rust:1.97-slim-bookworm` + `cargo install cargo-chef` |
| `planner` | Copies `Cargo.toml`, `Cargo.lock`, `common/`, `apps/backend/` and runs `cargo chef prepare` |
| `builder` | `cargo chef cook --release` for the dependency layer, then `cargo build --release --bin <service>` |
| `runtime` | `gcr.io/distroless/cc-debian12:nonroot`, only the release binary |

- `ENV APP_ENV=production` in the runtime stage
- `EXPOSE` matches the `PORT` the service reads
- The binary is the `ENTRYPOINT` (exec form) — distroless has no shell
- Never `COPY . .` — copy `Cargo.toml Cargo.lock common/ apps/backend/` only
- Release builds use `lto = "thin"`, `codegen-units = 1`, `strip` from the workspace `[profile.release]`; panics unwind so `catch_panic_layer` can return a 500
- Do not install system packages in the runtime stage; TLS is rustls so no OpenSSL is needed

## Correct EXPOSE values (do not copy from other sources)

| Service | `EXPOSE` |
|---|---|
| `apps/backend/api-gateway/Dockerfile` | `4000` |
| `apps/backend/admin-api/Dockerfile` | `4001` |
| `apps/backend/customer-api/Dockerfile` | `4002` |
| `apps/backend/schedule-api/Dockerfile` | `4003` |
| `apps/frontend/admin-web/Dockerfile` | `80` |
| `apps/frontend/customer-web/Dockerfile` | `3000` |

The 3000–3003 range is wrong for backend services and will cause Coolify port-detection to misfire.

## Healthcheck uses the binary itself

Distroless images have no `wget`, `curl` or shell. Every backend binary supports a `healthcheck` subcommand that performs a plain-TCP `GET http://127.0.0.1:$PORT<path>` (the service's ping/health path) and exits 0 or 1:

```dockerfile
HEALTHCHECK --interval=15s --timeout=5s --retries=3 \
  CMD ["/usr/local/bin/customer-api", "healthcheck"]
```

The same exec-form command is used as the `healthcheck.test` in `docker-compose.yaml`: `["CMD", "/usr/local/bin/customer-api", "healthcheck"]`.

## Migrations

`admin-api` owns the embedded SQLx migrations from `common/database/migrations`. The compose `migrate` one-shot service reuses the `admin-api` image with `command: ["migrate"]`, applies pending migrations and exits. Never run migrations from the application containers' normal startup.

## Next.js App Dockerfile (`customer-web`, standalone, EXPOSE 3000)

Next.js requires `outputFileTracingRoot` in `next.config.mjs` pointing at the monorepo root so `.next/standalone` preserves the correct nested paths.

```javascript
// apps/frontend/customer-web/next.config.mjs
import path from "path";

const nextConfig = {
  output: "standalone",
  outputFileTracingRoot: path.join(import.meta.dirname, "../../"),
};

export default nextConfig;
```

```dockerfile
FROM node:22-alpine AS base
ENV PNPM_HOME="/pnpm"
ENV PATH="$PNPM_HOME:$PATH"
RUN corepack enable

FROM base AS builder
WORKDIR /app
ENV CI=true
COPY pnpm-workspace.yaml pnpm-lock.yaml package.json turbo.json ./
COPY common/ ./common/
COPY apps/frontend/customer-web/ ./apps/frontend/customer-web/
RUN pnpm install --frozen-lockfile
ARG NEXT_PUBLIC_CUSTOMER_API_BASE_URL
ARG NEXT_PUBLIC_CUSTOMER_APP_NAME
ENV NEXT_PUBLIC_CUSTOMER_API_BASE_URL=$NEXT_PUBLIC_CUSTOMER_API_BASE_URL
ENV NEXT_PUBLIC_CUSTOMER_APP_NAME=$NEXT_PUBLIC_CUSTOMER_APP_NAME
RUN pnpm --filter customer-web build

FROM node:22-alpine AS runner
WORKDIR /app
ENV NODE_ENV=production
ENV PORT=3000
ENV HOSTNAME=0.0.0.0
COPY --from=builder /app/apps/frontend/customer-web/.next/standalone ./
COPY --from=builder /app/apps/frontend/customer-web/.next/static ./apps/frontend/customer-web/.next/static
COPY --from=builder /app/apps/frontend/customer-web/public ./apps/frontend/customer-web/public
EXPOSE 3000
CMD ["node", "apps/frontend/customer-web/server.js"]
```

The `COPY` paths for static files must reflect the nested monorepo path inside `.next/standalone`, not a flat structure. `NEXT_PUBLIC_*` args are declared before the build and must be set as **build args** in `.github/workflows/continuous-integration.yml` (matrix `build-args`) so they bake into the bundle — GitHub Actions builds this image, not Coolify (see below).

## React + Vite App Dockerfile (`admin-web`, EXPOSE 80)

```dockerfile
FROM node:22-alpine AS base
ENV PNPM_HOME="/pnpm"
ENV PATH="$PNPM_HOME:$PATH"
RUN corepack enable

FROM base AS builder
WORKDIR /app
ENV CI=true
COPY pnpm-workspace.yaml pnpm-lock.yaml package.json turbo.json ./
COPY common/ ./common/
COPY apps/frontend/admin-web/ ./apps/frontend/admin-web/
RUN pnpm install --frozen-lockfile
ARG VITE_ADMIN_API_BASE_URL
ARG VITE_ADMIN_APP_NAME
ENV VITE_ADMIN_API_BASE_URL=$VITE_ADMIN_API_BASE_URL
ENV VITE_ADMIN_APP_NAME=$VITE_ADMIN_APP_NAME
RUN pnpm --filter admin-web build

FROM nginx:alpine AS runner
COPY --from=builder /app/apps/frontend/admin-web/dist /usr/share/nginx/html
COPY infrastructure/nginx/admin-web.conf /etc/nginx/conf.d/default.conf
EXPOSE 80
CMD ["nginx", "-g", "daemon off;"]
```

`VITE_*` args are declared before the build and must be set as **build args** in GitHub Actions so they bake into the bundle. Frontend package names are **unscoped** (`admin-web`, `customer-web`).

## `.dockerignore`

Place at the monorepo root:

```
target
node_modules
.next
dist
.turbo
.git
*.log
.env
coverage
```

## Root docker-compose.yaml

This is the single Coolify deployment stack. Every deployable service is declared here. Do not create `docker-compose.prod.yml` or `docker-compose.qa.yml` — Coolify manages environments.

`devops/docker-compose.dev.yml` is for local development only, never deployed.

**Images are built by GitHub Actions and pushed to GHCR — Coolify never builds.** The committed root `docker-compose.yaml` declares every service as `image: ghcr.io/ru5ty-gate/<service>:main` with `pull_policy: always`; there is no `build:` block anywhere in that file. GitHub Actions builds each `apps/*/Dockerfile`, pushes `:main` and `:sha-<short-sha>` tags to GHCR, and Coolify's Docker Compose resource only pulls and swaps containers. See the `deployment-coolify` agent's "Image build strategy" section for the full flow.

## Container startup ordering — depends_on

`depends_on` in `docker-compose.yaml` controls **startup order**, not build order. The `migrate` one-shot service runs `admin-api migrate` and exits; `admin-api`/`customer-api`/`schedule-api` gate on it with `condition: service_completed_successfully`, and `api-gateway` gates on the APIs with `condition: service_healthy` so it never proxies to a not-yet-ready backend. Do not reference services outside this compose file (e.g. Coolify-managed Postgres/Redis) in `depends_on` — reach those only via `DATABASE_URL`/`REDIS_URL` env vars.

## Build-time environment

Frontend Dockerfiles must not set `NODE_ENV=production` as a build-time `ARG`/`ENV` before `pnpm install` — it skips `devDependencies` and breaks the build. Backend images set `APP_ENV=production` in the runtime stage only.

## What not to do

- Do not use `COPY . .` in a Dockerfile — copy only the workspace manifests, `common/` and the target app
- Do not create environment-specific compose files (`docker-compose.qa.yml`, etc.) — Coolify environments handle this
- Do not use `latest` image tags in production — use the `:main` / `:sha-<short-sha>` tags GitHub Actions pushes to GHCR
- Do not copy `.env` files into images — inject via Coolify Environment Variables (runtime) or GitHub Actions build args (`NEXT_PUBLIC_*`/`VITE_*`)
- Do not run apps as root in production — backend images use the distroless `nonroot` user
- Do not add a shell, `curl` or `wget` to backend runtime images for healthchecks — use the `healthcheck` subcommand
- Do not copy `EXPOSE` port values from other sources — use the table above
- Do not add a `build:` block back to the root `docker-compose.yaml` — it must stay `image:`-only so Coolify never builds on the VPS
- Frontend images: do not use `npm install -g pnpm` or `turbo prune` — use corepack + `pnpm deploy --prod --legacy`
