---
name: enterprise-scale
description: Use as a cross-cutting reference when designing for 1M+ concurrent users — caching strategy, queue offloading, database scale patterns, frontend performance, or API client resilience. Trigger on "scale this", "enterprise scale", "high traffic", or when reviewing whether a feature is production-ready for heavy load.
tools: Read, Grep, Glob
model: inherit
---

All code in this monorepo is designed for enterprise scale (1M+ concurrent users) by default. These are the cross-cutting concerns to apply.

## Backend services (Rust / Axum)

Stateless design — no in-memory sessions, no local file state, all state in Postgres or Redis. Horizontal scaling — every service runs behind a load balancer with N replicas, no singleton assumptions. The one deliberate exception is `schedule-api`'s recurring job loop: `process_deliveries` selects pending rows without a row claim, so run it as a single replica (or add `FOR UPDATE SKIP LOCKED` claiming before scaling it out).

Graceful shutdown is built into `serve` (SIGTERM and SIGINT drain in-flight requests). Do not add another handler.

Every service must expose `GET /health` (liveness) and `GET /ready` (readiness — only 200 if DB + Redis are healthy). Response compression is applied at the gateway or NGINX for bodies over 1KB. `x-correlation-id` is propagated through all service-to-service calls for distributed tracing.

Rate limiting: `RateLimitPolicy::per_minute` tiers — global 200/min, auth 10/min, sensitive 5/min per client, with per-route overrides through `route_layer`. The gateway limit is `RATE_LIMIT_MAX`.

Runtime: Tokio multi-thread runtime; never block it. CPU-heavy work (bcrypt, report generation) goes through `tokio::task::spawn_blocking`. Release builds use thin LTO, one codegen unit and stripped symbols. Panics unwind (no `panic = "abort"`), so `catch_panic_layer` converts a handler panic into a 500 response and the process keeps serving; clippy already denies `unwrap`/`expect`/`panic!` in our own code, so this only guards against panics inside dependencies.

## Database (SQLx + PostgreSQL)

Connection pooling via `DATABASE_CONNECTION_LIMIT` (10 dev, 50–100 prod per replica) and `DATABASE_POOL_TIMEOUT`. Read-write separation: add a second `PgPool` for a replica and pass it to read-heavy services explicitly. Query efficiency: always list columns on list/search queries, never fetch everything on paginated endpoints. Keyset (cursor) pagination for any customer-facing list that could exceed 10K rows — never large `OFFSET`. Optimistic locking via a `version` integer on concurrent-write entities (orders, inventory, carts) — `UPDATE ... WHERE id = $1 AND version = $2` and check `rows_affected`. Batch writes with `UNNEST` arrays or `QueryBuilder::push_values` instead of per-row loops.

## Caching (Redis)

Cache-aside on every read-heavy method: check Redis first, fall back to Postgres, then populate the cache with `set_json`. TTL by entity type: catalog/reference data 15 min, user profiles 5 min, config/settings 30 min, transactional data 1 min or skip cache entirely. Cache key namespacing: `{service}:{entity}:{id}`. Invalidation deletes specific keys AND the matching list keys (`keys_matching` + `del_many`). For ultra-hot keys, avoid cache stampede via short TTL plus a lock key (`SET NX`). Cache errors never fail a request.

## Queue and background work

Offload: email sending, PDF/report generation, image processing, webhook delivery, audit log batching, scheduled jobs (reminders, cleanups, analytics). Use `ru5ty-gate-queue` (Redis ZSET queue and worker) or a database-backed table with a scheduled job (as webhook deliveries do). Jobs must be idempotent — safe to retry on failure.

## Frontend (React / Next.js)

Code splitting via `React.lazy()` and `Suspense` on routes and heavy components. Virtual scrolling (`react-window` or `@tanstack/virtual`) for lists over 100 items. Debounced search inputs, 300ms minimum, before triggering API calls. Optimistic UI on mutations — update immediately, roll back on error. Service worker for offline resilience on customer-facing apps where it makes sense. Bundle size monitored via Vite `rollupOptions` manual chunks, initial JS under 200KB gzipped. Image optimization via Next.js `<Image>` (customer-web) or lazy-loaded `<img loading="lazy">` (admin-web). Error boundaries wrap every page and major feature section with retry UI.

## API client (Axios)

Retry with exponential backoff — 3 retries at 1s/2s/4s on 5xx and network errors. Request deduplication handled by React Query — never bypass with raw Axios in components. On a 503, show degraded UI with a retry option rather than hammering the endpoint.

## Environment variables (scale-related)

Backend services read, with defaults where noted:

```
DATABASE_CONNECTION_LIMIT=10
DATABASE_POOL_TIMEOUT=10
RATE_LIMIT_MAX=200
SCHEDULE_WEBHOOK_INTERVAL_SECONDS=60
```

Add a new variable to the service's `ServiceConfig`, to its `.env.example`, and to `docker-compose.yaml` together; never read it ad hoc with `std::env::var`.
