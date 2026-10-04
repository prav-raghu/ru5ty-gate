# Sentry Wiring Guide

This guide covers how to wire up Sentry error monitoring in any project that was forked from this template. The `common/observability` package is already built — this is purely the integration steps per service.

Sentry is **disabled automatically when `SENTRY_DSN` is not set**, so local development needs no configuration.

---

## Template status

All four backend services in this template are wired. When you fork the template, only the crate prefix changes (`ru5ty-gate-observability`).

| Service | Wired |
|---|---|
| `api-gateway` | yes |
| `admin-api` | yes |
| `customer-api` | yes |
| `schedule-api` | yes |

---

## Wiring a backend service (copy-paste checklist)

### 1. Add the dependency

In the service's `Cargo.toml`:

```toml
ru5ty-gate-observability = { version = "1.0.0", path = "../../../common/observability" }
```

---

### 2. `main.rs` — initialise Sentry first and keep the guard alive

```rust
let env = EnvReader::from_process();
let _sentry = init_sentry(&env);
```

`init_sentry` returns `None` when `SENTRY_DSN` is unset. Keep the returned guard in a variable for the lifetime of `main` so buffered events are flushed on shutdown, and call it before logging and the server start so panics during startup are captured too (the panic integration is enabled).

---

### 3. 5xx errors are captured centrally

Nothing to add per handler. `AppError::Internal` in `common/http` logs the error once and calls `capture_error` when it is turned into a response, so every service that returns `AppError::internal(...)` for unexpected failures reports to Sentry automatically. Handler panics are captured by the panic integration and returned as 500 by `catch_panic_layer`.

Only unexpected failures use `AppError::Internal`; 4xx errors (validation, auth, not found) never reach Sentry because they are not bugs.

---

### 4. Add Sentry vars to `.env.example`

```dotenv
# Sentry — leave empty to disable error monitoring (local dev)
SENTRY_DSN=
SENTRY_RELEASE=
SENTRY_TRACES_SAMPLE_RATE=0.1
```

---

### 5. Set `SENTRY_DSN` in Coolify per app

Each Coolify application gets its own backend DSN from your Sentry project → Settings → Client Keys. Paste the DSN into the app's environment variables panel in Coolify.

`SENTRY_DSN` blank → Sentry is disabled (safe for local dev and CI).

---

## Environment variable reference

| Variable | Where set | Notes |
|---|---|---|
| `SENTRY_DSN` | Coolify env panel | Empty string disables Sentry entirely. |
| `APP_ENV` | Coolify env panel | Used as the Sentry environment tag (`production` or `development`). |
| `SENTRY_RELEASE` | Coolify env panel or CI | Optional. Tag errors with a release version or git SHA. |
| `SENTRY_TRACES_SAMPLE_RATE` | Coolify env panel | Float `0.0`–`1.0`. Defaults to `0.1` (10% of traces). |

---

## Frontend wiring (separate packages — not via `common/observability`)

### `admin-web` (Vite + React)

Install:

```bash
pnpm add @sentry/browser @sentry/vite-plugin
```

`src/main.tsx` — before `ReactDOM.createRoot`:

```ts
import * as Sentry from '@sentry/browser';

Sentry.init({
  dsn: import.meta.env.VITE_SENTRY_DSN,
  environment: import.meta.env.MODE,
  tracesSampleRate: 0.1,
});
```

`vite.config.ts` — add the plugin for source map upload (only active when `SENTRY_AUTH_TOKEN` is set):

```ts
import { sentryVitePlugin } from '@sentry/vite-plugin';

export default defineConfig({
  plugins: [
    react(),
    sentryVitePlugin({
      org: process.env.SENTRY_ORG,
      project: process.env.SENTRY_PROJECT,
      authToken: process.env.SENTRY_AUTH_TOKEN,
    }),
  ],
  build: { sourcemap: true },
});
```

`.env.example`:

```dotenv
VITE_SENTRY_DSN=
SENTRY_ORG=
SENTRY_PROJECT=
SENTRY_AUTH_TOKEN=
```

---

### `customer-web` (Next.js)

Install:

```bash
pnpm add @sentry/nextjs
```

Run the wizard (optional but generates the config files):

```bash
npx @sentry/wizard@latest -i nextjs
```

Or add manually:

`sentry.client.config.ts`:

```ts
import * as Sentry from '@sentry/nextjs';

Sentry.init({
  dsn: process.env.NEXT_PUBLIC_SENTRY_DSN,
  tracesSampleRate: 0.1,
});
```

`sentry.server.config.ts` and `sentry.edge.config.ts` — same content, same `dsn`.

`next.config.ts`:

```ts
import { withSentryConfig } from '@sentry/nextjs';

const nextConfig = { /* your existing config */ };

export default withSentryConfig(nextConfig, {
  org: process.env.SENTRY_ORG,
  project: process.env.SENTRY_PROJECT,
  authToken: process.env.SENTRY_AUTH_TOKEN,
  silent: true,
});
```

`.env.example`:

```dotenv
NEXT_PUBLIC_SENTRY_DSN=
SENTRY_ORG=
SENTRY_PROJECT=
SENTRY_AUTH_TOKEN=
```

---

## Summary of what `common/observability` exports

```rust
use ru5ty_gate_observability::{init_sentry, capture_error, resolve_sentry_config};
// init_sentry(&EnvReader) -> Option<ClientInitGuard>   call once in main.rs
// capture_error(&dyn Error)                            used by AppError for 5xx
// resolve_sentry_config(&EnvReader)                    rarely needed directly
```
