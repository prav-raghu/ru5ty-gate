---
description: Initialize this monorepo template for a new project — rename all namespace references, generate secrets, and create .env files
argument-hint: <project slug, e.g. "burger-shop"> [org scope, e.g. "zynkosi"]
---

Initialize this monorepo for a new project: $ARGUMENTS

Parse the arguments:
- First argument = project slug (lowercase, hyphenated, e.g. `burger-shop`)
- Second argument (optional) = org prefix (e.g. `zynkosi` → scope becomes `@zynkosi/burger-shop`). If not provided, scope = `@{slug}` (e.g. `@burger-shop`)

If arguments are missing, ask the user for the project slug before proceeding.

## Step 1 — Confirm before touching anything

Show the user exactly what will change:
- Old namespace: `ru5ty-gate` (kebab, Cargo package names and docs) / `ru5ty_gate` (snake, Rust `use` paths)
- New namespace: `{slug}` / `{slug_snake}` (the slug with hyphens replaced by underscores)
- Files to update: all .rs, .toml, .lock (Cargo.lock), .ts, .tsx, .json, .md, .yaml, .yml, .properties, Dockerfile files containing the old names
- .env files to create: 9 files from their .env.example templates
- Post-steps: pnpm install, cargo check, typecheck

Wait for the user to confirm before proceeding.

## Step 2 — Rename namespace in all source files

Use Bash to find and replace in all relevant files in one pass. Exclude `node_modules/`, `.git/`, `dist/`, `pnpm-lock.yaml`, and `apps/cms/package-lock.json`.

**Two substitutions, applied in this order:**

1. Replace the literal string `ru5ty_gate` with `{slug_snake}` (Rust `use` paths and library crate names)
2. Replace the literal string `ru5ty-gate` with `{slug}` in a single pass — this handles Cargo package names, path dependencies, image names, and any `@ru5ty-gate/` scoped frontend package at once

```bash
find . \
  -type f \
  \( -name "*.rs" -o -name "*.toml" -o -name "Cargo.lock" \
     -o -name "*.ts" -o -name "*.tsx" -o -name "*.json" -o -name "*.md" \
     -o -name "*.yaml" -o -name "*.yml" -o -name "*.properties" \
     -o -name "*.mjs" -o -name "*.sh" -o -name "Dockerfile" \) \
  ! -path "*/node_modules/*" \
  ! -path "*/target/*" \
  ! -path "*/.git/*" \
  ! -path "*/dist/*" \
  ! -name "pnpm-lock.yaml" \
  ! -path "*/apps/cms/package-lock.json" \
  -exec grep -lE "ru5ty[-_]gate" {} \; | \
  xargs sed -i -e 's/ru5ty_gate/{slug_snake}/g' -e 's/ru5ty-gate/{slug}/g'
```

If the org scope differs from the slug (e.g., `@zynkosi/burger-shop` vs just `@burger-shop`), run a second pass over the frontend `package.json` files only, replacing `@{slug}/` → `@{org-prefix}/{slug}/`. Rust crate names do not use the org scope.

After running, verify a sample of replaced files to confirm correctness.

## Step 3 — Update CLAUDE.md project section

Replace the "Using this as a template" section at the bottom of CLAUDE.md with:

```markdown
## Project: {Display Name}

This is the {Display Name} project, initialized from ru5ty-gate.
Package scope: `{scope}` (frontend); Rust crates: `{slug}-*`
Initialized: {date}
```

## Step 4 — Generate secrets

Generate these values with OpenSSL (run inline with Bash):

```bash
echo "JWT_SECRET=$(openssl rand -hex 64)"; echo "JWT_REFRESH_SECRET=$(openssl rand -hex 64)"; echo "TWO_FACTOR_KEY=$(openssl rand -hex 32)"; echo "SCHEDULE_API_KEY=$(openssl rand -hex 32)"
```

Store the generated values — use them in Step 5.

## Step 5 — Create .env files from examples

For each `.env.example` file, copy it to `.env` and apply these substitutions:

| Placeholder | Replace with |
|---|---|
| `ru5ty_gate` (in DATABASE_URL) | `{slug_snake}` |
| `REPLACE_WITH_64_BYTE_HEX_SECRET` | generated JWT_SECRET |
| `REPLACE_WITH_64_BYTE_HEX_SECRET` (second) | generated JWT_REFRESH_SECRET |
| `REPLACE_WITH_32_BYTE_HEX_KEY_64_CHARS` | generated TWO_FACTOR_KEY |
| `your-secret-api-key-minimum-32-chars-here` | generated 32-byte hex |
| `your_mailtrap_api_key` | `dev_mailtrap_key_replace_me` |
| `noreply@yourdomain.com` | `noreply@{slug}.local` |
| `Your App Name` | `{Display Name}` |

Create .env files at:
- `apps/backend/api-gateway/.env`
- `apps/backend/admin-api/.env`
- `apps/backend/customer-api/.env`
- `apps/backend/schedule-api/.env`
- `apps/frontend/admin-web/.env`
- `apps/frontend/customer-web/.env`
- `apps/mobile/customer-mobile/.env`
- `common/database/.env`
- `devops/.env`

## Step 6 — Install and generate

```bash
pnpm install
cargo check --workspace --all-targets
```

## Step 7 — Verify

```bash
pnpm typecheck
cargo clippy --workspace --all-targets -- -D warnings
```

Fix any errors before marking complete.

## Step 8 — Report

Show the user:
- Files updated (count)
- .env files created (list)
- Secrets generated (names only, never values)
- Any typecheck errors that need attention
- Next steps: start the local stack with `docker compose -f devops/docker-compose.dev.yml up -d`, have the developer run `./devops/scripts/migrate.sh`, then start the backend with `./devops/scripts/dev.sh`
