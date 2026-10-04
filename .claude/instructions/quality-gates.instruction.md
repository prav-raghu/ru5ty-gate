# Quality Gates Configuration

This document outlines the quality gates and standards enforced in this project.

## Rust Quality Rules (backend and `common/`)

Configured once in the root `Cargo.toml` under `[workspace.lints]` and inherited by every crate with `[lints] workspace = true`.

| Rule                                            | Level  | Description                                          |
| ----------------------------------------------- | ------ | ---------------------------------------------------- |
| `unsafe_code`                                   | forbid | No `unsafe` anywhere                                 |
| `clippy::pedantic`                              | warn   | Pedantic lint group; CI runs with `-D warnings`      |
| `clippy::unwrap_used` / `expect_used`           | deny   | Errors are propagated, never unwrapped               |
| `clippy::panic` / `todo` / `unimplemented`      | deny   | No panics or placeholders in production code         |
| `clippy::dbg_macro`                             | deny   | No debug macros left behind                          |

Allowed pedantic exceptions (documented in the root `Cargo.toml`): `module_name_repetitions`, `missing_errors_doc`, `missing_panics_doc`, `must_use_candidate`, `cast_precision_loss`, `struct_excessive_bools`, `match_same_arms`, `unused_async`, `redundant_closure_for_method_calls`, `single_match_else`, `duration_suboptimal_units`.

Test files opt out of the unwrap/expect/panic denials with `#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]` at the top of the file. `clippy.toml` also sets `allow-unwrap-in-tests`, `allow-expect-in-tests` and `allow-panic-in-tests` for inline `#[cfg(test)]` modules.

Formatting is `rustfmt` with `rustfmt.toml` (edition 2024, `max_width = 100`).

Supply chain: `cargo deny check` with `deny.toml` (advisories, licences, bans, sources) runs in CI.

## ESLint Quality Rules (frontend and mobile TypeScript)

### TypeScript Rules

| Rule                            | Severity | Description                              |
| ------------------------------- | -------- | ---------------------------------------- |
| `no-explicit-any`               | Error    | Prevents usage of `any` type             |
| `explicit-function-return-type` | Warn     | Requires explicit return types           |
| `no-unused-vars`                | Error    | Flags unused variables                   |
| `consistent-type-imports`       | Error    | Enforces `type` imports                  |
| `no-floating-promises`          | Error    | Requires handling promises               |

### Code Quality (SonarJS)

| Rule                     | Threshold | Description                                 |
| ------------------------ | --------- | ------------------------------------------- |
| `cognitive-complexity`   | 15        | Maximum cognitive complexity per function   |
| `no-duplicate-string`    | 3         | Maximum duplicate strings before extraction |
| `no-identical-functions` | Warn      | Flags identical function bodies             |

### Import Organization

- Groups: builtin → external → internal → parent/sibling → index → type
- Alphabetical ordering within groups
- No duplicate imports

## Test Coverage

| Metric     | Minimum (frontend) | Rust                                             |
| ---------- | ------------------ | ------------------------------------------------ |
| Branches   | 80%                | Reported by `cargo llvm-cov` for SonarCloud      |
| Functions  | 80%                | Every service layer has `tests/services` tests   |
| Lines      | 80%                | No local threshold gate                          |
| Statements | 80%                | —                                                |

Coverage thresholds are separate, optional checks and are not part of the three mandatory gates (typecheck, lint, format).

## SonarCloud Quality Gate

Default "Sonar way" quality gate requires:

### On New Code (PRs)

| Metric                     | Condition |
| -------------------------- | --------- |
| Coverage                   | ≥ 80%     |
| Duplicated Lines           | ≤ 3%      |
| Maintainability Rating     | A         |
| Reliability Rating         | A         |
| Security Rating            | A         |
| Security Hotspots Reviewed | 100%      |

### Overall Code

| Metric                 | Condition |
| ---------------------- | --------- |
| Coverage               | ≥ 80%     |
| Duplicated Lines       | ≤ 3%      |
| Maintainability Rating | A         |
| Reliability Rating     | A         |
| Security Rating        | A         |

## Commit Message Convention

Format: `type(scope): description`

### Types

- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation
- `style`: Code style (formatting)
- `refactor`: Code refactoring
- `perf`: Performance improvement
- `test`: Adding/updating tests
- `build`: Build system changes
- `ci`: CI configuration
- `chore`: Maintenance tasks
- `revert`: Reverting changes

### Scopes

All package and crate names are valid scopes:

- Backend: `api-gateway`, `customer-api`, `admin-api`, `schedule-api`
- Frontend: `customer-web`, `admin-web`, `customer-mobile`
- Common: `auth`, `database`, `cache`, `config`, `email`, `export`, `http`, `logging`, `metrics`, `observability`, `queue`, `sms`, `storage`, `types`, `utilities`, `webhooks`
- Meta: `deps`, `ci`, `docs`, `release`

## Pre-commit Checks

Automated via Husky + lint-staged:

1. **Staged Rust files:**
   - `cargo fmt --all`
   - `cargo clippy --workspace --all-targets --locked -- -D warnings`
   - Must pass with zero warnings

2. **Staged TypeScript/JavaScript files:**
   - ESLint with auto-fix
   - Prettier formatting
   - Must pass with zero warnings

3. **Typecheck:**
   - `pnpm typecheck` (Turbo typecheck for the frontends plus `cargo check --workspace --all-targets --locked`)

4. **Commit message:**
   - Must follow conventional commit format
   - Type must be from allowed list
   - Subject must be lowercase

5. **Pre-push:**
   - Changeset status check (warns if no changeset)
