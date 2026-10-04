# Common Crate Environment Configuration

Reference for the `EnvReader` pattern used across the Rust crates, and the Redis TLS behaviour it drives. Linked from `.claude/commands/deploy-coolify.md`.

## The pattern

Every crate or service that reads environment variables does so through `EnvReader` in `common/config` and a typed config struct (see `.claude/instructions/env-config.instructions.md` for the full pattern and the service-level version of this convention). Each crate only declares the variables it actually needs, in its own `*_config.rs`.

Example, `common/cache`:

```rust
let url = env.required("REDIS_URL")?;
let reject_unauthorized = env
    .optional("REDIS_TLS_REJECT_UNAUTHORIZED")
    .is_some_and(|value| value == "true");
```

`REDIS_URL` is required: there is no discrete `REDIS_HOST`/`REDIS_PORT`/`REDIS_PASSWORD` fallback, and a missing value stops the service at startup instead of silently connecting to `127.0.0.1:6379`. Local development still needs a real `REDIS_URL` in `.env`; point it at `devops/docker-compose.dev.yml`'s Redis container, for example `redis://localhost:6379`.

## Redis TLS behaviour (`common/cache`)

`RedisService::connect(url, reject_unauthorized)` decides whether to use TLS from the connection string scheme, not a separate flag:

- A `rediss://` URL (double "s") turns TLS on automatically. This is what Coolify's managed Redis resource uses.
- `REDIS_TLS_REJECT_UNAUTHORIZED` controls certificate validation and is only true when set to the string `true`. It must be `false` for Coolify-managed Redis, which presents a self-signed certificate. `docker-compose.yaml` therefore sets `REDIS_TLS_REJECT_UNAUTHORIZED: ${REDIS_TLS_REJECT_UNAUTHORIZED:-false}` for every backend service.
- Set `REDIS_TLS_REJECT_UNAUTHORIZED=true` only when a CA-signed Redis certificate is in use.

If the connection cannot be established the service logs a warning and runs with a disabled cache (`RedisService::disabled()`, every operation a no-op) rather than crashing. Authentication features that need Redis (token blacklist, refresh-token allow-list, lockouts) fail closed, so production must run with a working Redis.

## Diagnosing connection errors

Errors surface in this order as each layer gets fixed, per `.claude/commands/deploy-coolify.md`'s "Managed Redis uses TLS with a self-signed cert" section:

1. `missing required environment variable REDIS_URL`: `REDIS_URL` is not set in the environment at all.
2. A DNS failure for the random Coolify hostname: the service is not on the same Docker network as the managed Redis resource (it needs the `coolify` external network).
3. `invalid peer certificate` or `UnknownIssuer`: the network is correct, but TLS validation is rejecting Coolify's self-signed certificate (`REDIS_TLS_REJECT_UNAUTHORIZED` needs to be `false`).
4. Normal request handling with cache hits: all three layers are correct.
