# CI/CD

How code gets from a push to the VPS. The workflow is `.github/workflows/continuous-integration.yml`; the Coolify side is described in `.claude/agents/deployment-coolify.md`.

## Pipeline

1. **install** - pnpm install for the frontends and tooling.
2. **lint** - `pnpm format:check`, `pnpm lint` (ESLint plus `cargo clippy -D warnings`) and `cargo deny`.
3. **typecheck** - `pnpm typecheck` (`tsc` for the frontends and `cargo check --workspace --all-targets --locked`).
4. **build** - `pnpm build` for the frontends and `cargo build --workspace --locked`.
5. **test** - frontend tests with coverage, then `pnpm test:rust` against Postgres 16 and Redis 7 service containers (`DATABASE_URL`, `TEST_REDIS_URL`).
6. **quality-gate** - passes only when lint, typecheck, build and test all pass.
7. **sonarcloud** - analyses the frontend and mobile TypeScript.
8. **changes** - path filter that decides which images to rebuild; `Cargo.toml`, `Cargo.lock`, `common/**` and the pnpm manifests are shared inputs.
9. **docker-build** - builds each changed image from its own Dockerfile (repo root as context) and pushes `:main`, `:sha-<short>` and, on release tags, the version tag to GHCR. Build cache is per service (`type=gha`).
10. **deploy** - triggers Coolify, which pulls the new images.

## Backend images

Backends are four-stage `cargo-chef` builds ending in `gcr.io/distroless/cc-debian12:nonroot`. Each binary implements a `healthcheck` subcommand used by the Dockerfile `HEALTHCHECK` and the compose healthcheck, because distroless has no shell or `curl`. Details: `.claude/rules/docker.md`.

## Migrations

The compose `migrate` service reuses the `admin-api` image and runs its `migrate` subcommand before any API starts. There is no post-deploy command. Details: `.claude/agents/database-migrations.md`.

## Rules

- Never weaken a gate (`continue-on-error` on lint or typecheck).
- Backend env vars are read through `EnvReader`; add new ones to `.env.example`, `docker-compose.yaml` and the service's `ServiceConfig` together.
- Adding a service means a Dockerfile, a compose entry, a `docker-build` matrix entry and a `changes` filter entry.
