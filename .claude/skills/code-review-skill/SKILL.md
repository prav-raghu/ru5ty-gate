---
description: Full code review — type safety, naming, security, project standards, and form validation coverage. Use after significant changes before considering a task done.
disable-model-invocation: true
argument-hint: <scope, e.g. "apps/backend/customer-api/src/routes" or "recent changes">
---

# Code Review: $ARGUMENTS

## Files to review

!`git diff --name-only HEAD~1 2>/dev/null | grep -E '\.(rs|ts|tsx)$' || find $ARGUMENTS -name "*.rs" -o -name "*.ts" -o -name "*.tsx" 2>/dev/null | head -30`

## Blockers (must fix before merge)

- `any` type used anywhere (TypeScript), or `unsafe`/`unwrap`/`expect`/`panic!`/`todo!`/`dbg!` in non-test Rust code
- `#[allow(...)]` added to production Rust code to silence a lint
- Hardcoded secret, API key, token, or connection string
- Auth bypass via query param, header, or env flag
- Empty `catch` block (TypeScript) or an error arm that silently discards a `Result` (Rust)
- Unused variables or imports
- Direct Axios call inside a component or `useEffect`
- `localStorage` used for a token
- `alert()` or `confirm()` in frontend code
- Native HTML form validation attributes (`required`, `pattern`, `type="email"` relied on for validation)
- Zod used on the backend
- `as` cast used to silence a type error
- `@ts-ignore` or `@ts-expect-error`
- Frontend form showing server 400/409 error as an inline field error instead of toast
- Frontend form showing Zod client error as a toast instead of inline field error
- Rust request struct missing `#[serde(deny_unknown_fields)]`
- Rust request struct missing `length(min = 1)` on a required string field
- `validator` max lengths not matching `VARCHAR(N)` in the migration
- `UNIQUE` column returning 400 on duplicate instead of 409
- SQL built with `format!` and user input
- TypeScript errors, `cargo clippy -- -D warnings` findings, or failing tests present

## Warnings (should fix)

- TypeScript class method missing explicit access modifier
- TypeScript DTO defined as a class instead of an interface
- Async function missing error handling
- Rust file defining more than one public item, or a controller containing business logic
- `unknown` narrowed without a type guard
- Hardcoded API URL or port number
- Missing loading, error, or empty state on a frontend data page
- Backend service missing `/health` or `/ready` endpoint
- List query using `SELECT *` instead of explicit columns

## Naming check

| Element | Expected convention |
|---|---|
| TS variables / functions / methods | `camelCase` |
| Rust functions / methods / variables / files | `snake_case` |
| Classes, structs, enums, traits | `PascalCase` |
| Interfaces | `PascalCase`, no `I` prefix |
| Rust route / schema / DTO files | `{entity}_route.rs`, `{entity}_schema.rs`, `{entity}_dto.rs` |
| DB tables / columns | `snake_case` |
| Frontend components | `PascalCase` |
| Frontend hooks | `use` prefix + `camelCase` |
| Frontend constants | `UPPER_SNAKE_CASE` |

## Security spot-check

- JWT validation in each service's `authenticate` layer (scope, blacklist, `minIat`)
- Rate limiting on all non-health routes
- Security headers layer applied on every Rust service
- `validator` runs (through `ValidatedJson`) before controller logic
- No PII in logs

## TypeCheck command

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test -p ru5ty-gate-<crate>
pnpm --filter <package-name> typecheck
```

Run the commands that match what changed. A review is not complete if any fails.

## Output format

**Blockers:** [list with file + line]
**Warnings:** [list with file + line]
**Suggestions:** [list]
**Verdict:** Pass / Needs fixes
