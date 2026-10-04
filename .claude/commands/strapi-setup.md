# Strapi CMS Setup

## The Problem

Strapi (`apps/cms`) cannot be managed by pnpm as a workspace package. Two issues arise when it is included in `pnpm-workspace.yaml`:

1. **Native module compilation** — Strapi's `better-sqlite3` dependency requires a native build step. pnpm's `onlyBuiltDependencies` allowlist would need to include it, and even then pnpm can fail to build it correctly on Windows.
2. **Peer dependency conflicts** — Strapi 5.x requires `react@^17 || ^18` as a peer dependency. If the workspace root or other packages declare React 19, pnpm hoists React 19 into Strapi's node_modules and Strapi's internal admin panel breaks.

Strapi ships with its own `package-lock.json` and is designed to be managed with **npm**, not pnpm.

## Solution Applied to This Template

### 1. Remove CMS from pnpm workspace

`pnpm-workspace.yaml` — remove the `apps/cms/*` entry:

```yaml
packages:
  - apps/backend/*
  - apps/frontend/*
  - apps/mobile/*
  # apps/cms is excluded — managed independently with npm
  - common/*
```

### 2. Pin React to 18 in `apps/cms/package.json`

Strapi 5 does not support React 19. Pin the versions it installs for its own admin panel:

```json
"react": "^18.3.1",
"react-dom": "^18.3.1",
"react-router-dom": "^6.27.0"
```

> `react-router-dom` must be v6 — Strapi's admin panel uses RRDv6 internally. Do not bump to v7.

### 3. Set a non-default port in `apps/cms/.env`

Strapi defaults to port `1337`. This conflicts with nothing in the template, but to keep all app ports grouped together, this project uses `4006`:

```env
HOST=0.0.0.0
PORT=4006
```

Update the fallback default in `apps/cms/config/server.ts` to match:

```ts
export default ({ env }) => ({
  host: env('HOST', '0.0.0.0'),
  port: env.int('PORT', 4006),
  ...
});
```

Update `.env.example` to match:

```env
PORT=4006
```

### 4. Root dev script uses `npm --prefix`

`package.json` root scripts:

```json
"dev:cms": "npm --prefix apps/cms install --legacy-peer-deps && npm --prefix apps/cms run develop"
```

The `--legacy-peer-deps` flag is a safety net for any transitive peer conflicts Strapi pulls in. The `install` step is intentionally left in the script so the first run is self-bootstrapping.

### 5. VS Code launch config

The CMS launch configuration in `.vscode/launch.json` must:

- Use `npm` as `runtimeExecutable` (not `pnpm`)
- Set `cwd` to `${workspaceFolder}/apps/cms` so Strapi reads its own `.env`
- Run `develop` (not `dev`) — Strapi's hot-reload script is named `develop`

```json
{
    "name": "📰 CMS (Strapi)",
    "type": "node",
    "request": "launch",
    "runtimeExecutable": "npm",
    "runtimeArgs": ["run", "develop"],
    "cwd": "${workspaceFolder}/apps/cms",
    "console": "integratedTerminal",
    "skipFiles": ["<node_internals>/**", "**/node_modules/**"],
    "presentation": {
        "group": "Backend Services",
        "order": 4
    }
}
```

Add `"📰 CMS (Strapi)"` to the `🛠️ Admin Stack` and `💻 Full Dev Environment` compounds.

### 6. Fix `tsconfig.json` module/moduleResolution mismatch

Strapi's generated `tsconfig.json` sets `"moduleResolution": "Node16"` but leaves `"module": "CommonJS"`. TypeScript 5.x enforces that these must match — set both to `Node16`:

```json
{
  "compilerOptions": {
    "module": "Node16",
    "moduleResolution": "Node16"
  }
}
```

## First-Run Setup

Because `apps/cms` is excluded from the pnpm workspace, `pnpm install` from the repo root will **not** install Strapi's dependencies. Run this once after cloning:

```bash
cd apps/cms
npm install --legacy-peer-deps
```

Start Strapi via VS Code (`📰 CMS (Strapi)` launch config) or from the terminal:

```bash
pnpm dev:cms
```

On first start, Strapi will prompt you to create an admin account at `http://localhost:4006/admin`.

Once logged in, generate an API token under **Settings → API Tokens**. The token type must be **Full Access** or a **Custom** token with the `Upload` plugin actions enabled (`upload`, `find`, `findOne`). A Read-only token will return `403` on every upload attempt.

Set the token in `apps/backend/admin-api/.env`:

```env
STRAPI_URL="http://localhost:4006"
STRAPI_API_TOKEN="your_token_here"
```

**Restart the admin API** after adding the token. `EnvConfig` reads and caches environment variables at process startup — changes to `.env` are not hot-reloaded. Until the process restarts, the upload endpoint will return `503 Media service not configured` even with the token present in the file.

### Diagnosing upload failures

If uploads still fail after restarting, check the admin-api logs for the Strapi response status:

| Status | Cause | Fix |
| ------ | ----- | --- |
| `503` | `STRAPI_API_TOKEN` empty or admin-api not restarted | Restart admin-api |
| `401` | Token value is invalid or malformed | Regenerate token in Strapi |
| `403` | Token exists but lacks upload permission | Change token type to Full Access |
| `404` | Wrong `STRAPI_URL` or Strapi not running | Verify Strapi is up on port 4006 |
| `502` | Strapi returned an unexpected error | Check Strapi terminal for stack trace |

## Database

Strapi uses **SQLite by default** in development — no database setup required. The file is created automatically at `apps/cms/.tmp/data.db` on first start.

For production, set `DATABASE_CLIENT=postgres` and provide a connection string. This must be a **separate** Postgres database from the main app — do not point it at `ru5ty_gate`:

```env
DATABASE_CLIENT=postgres
DATABASE_URL=postgresql://user:password@host:5432/strapi_db
```

The `.tmp/` directory is gitignored. Deleting `data.db` resets Strapi to a blank state (all content types, media, and tokens are lost).

## Port Reference

| Service       | Port |
|---------------|------|
| Admin API       | 4001 |
| Admin Web (dev) | 4004 (`VITE_ADMIN_PORT`, see `apps/frontend/admin-web/vite.config.ts`) |
| Admin Web (prod)| 80 (nginx, Traefik-routed) |
| CMS (Strapi)    | 4006 |
