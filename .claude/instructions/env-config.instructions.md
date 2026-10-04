---
applyTo: "apps/backend/**/config/**/*.rs,common/**/src/*config*.rs"
description: "Environment variable validation — typed ServiceConfig built from EnvReader in every service and common crate"
---

# Environment Variable Configuration

Every backend service and `common/*` crate that reads environment variables must do so through the `EnvReader` from `ru5ty-gate-config`, and convert the result into a typed config struct once at startup. Never call `std::env::var` in service, controller or library code.

## Pattern

`EnvReader` (in `common/config/src/env_reader.rs`) wraps a map of variables. `EnvReader::from_process()` is called once in `main.rs`; tests build one with `EnvReader::from_pairs([...])`.

| Method | Behaviour |
|---|---|
| `optional(key) -> Option<String>` | trimmed value, `None` when missing or blank |
| `required(key) -> Result<String, ConfigError>` | `ConfigError::Missing` when absent or blank |
| `parse_required::<T>(key)` | required and parsed with `FromStr` (`ConfigError::Invalid` on failure) |
| `parse_or(key, default)` | parsed value, or the default when absent |

Each service has `src/config/service_config.rs`:

```rust
#[derive(Debug, Clone)]
pub struct ServiceConfig {
    pub port: u16,
    pub production: bool,
    pub cors_origin: String,
    pub redis_url: String,
    pub database: DatabaseConfig,
    pub auth: AuthConfig,
    pub email: EmailConfig,
}

impl ServiceConfig {
    pub fn from_env(env: &EnvReader) -> Result<Self, ServiceConfigError> {
        let environment = env
            .optional("APP_ENV")
            .unwrap_or_else(|| "development".to_owned());
        let production = match environment.as_str() {
            "production" => true,
            "development" => false,
            other => return Err(ServiceConfigError::InvalidEnvironment(other.to_owned())),
        };
        Ok(Self {
            port: env.parse_required("PORT")?,
            production,
            cors_origin: env.required("CORS_ORIGIN")?,
            redis_url: env.required("REDIS_URL")?,
            database: DatabaseConfig::from_env(env)?,
            auth: AuthConfig::from_env(env, TokenScope::Customer)?,
            email: EmailConfig::from_env(env)?,
        })
    }
}
```

Shared concerns have their own config in the owning crate and are composed in: `DatabaseConfig::from_env` (`DATABASE_URL`, `DATABASE_CONNECTION_LIMIT`, `DATABASE_POOL_TIMEOUT`), `AuthConfig::from_env` (`JWT_SECRET`, `JWT_REFRESH_SECRET`, both at least 32 characters), `EmailConfig::from_env` (`MAILTRAP_*`).

## Usage

`main.rs` builds the config and exits non-zero on failure:

```rust
let env = EnvReader::from_process();
let config = match ServiceConfig::from_env(&env) {
    Ok(config) => config,
    Err(error) => {
        tracing::error!(%error, "invalid configuration");
        return ExitCode::FAILURE;
    }
};
```

Services receive what they need through `AppState.config` (`Arc<ServiceConfig>`) or through constructor arguments in `plugins/services.rs`. They never re-read the environment.

## Rules

- `ServiceConfig::from_env` is called once at startup — if required vars are missing or malformed, the process exits before binding a port
- Extend the struct per service to include service-specific vars (for example `ADMIN_BOOTSTRAP_ENABLED`, `SCHEDULE_API_KEY`, `CUSTOMER_API_URL`)
- `APP_ENV` is `development` or `production`; any other value stops the service at startup
- Never use `std::env::var("X").unwrap_or(...)` outside `EnvReader::from_process` — defaults live in `parse_or`/`unwrap_or_else` inside `ServiceConfig`
- Secrets are validated for strength in their config type (minimum length), not at the point of use
- Every required var must be in both `.env` (real values, gitignored) and `.env.example` (placeholders, committed), and in `docker-compose.yaml` for deployed services
- Add a unit test in `tests/unit/service_config.rs` for a complete environment and for each missing required value
