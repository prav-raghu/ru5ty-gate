---
description: Add backend API endpoints for an existing database table — generates request schema, DTO, service, controller, and route
argument-hint: <model name and target service, e.g. "product endpoints in customer-api">
---

Use the backend-service subagent to generate complete backend API endpoints for: $ARGUMENTS

1. Read the migration and the Rust model to understand the table's columns and relations
2. Create `validator` request structs (with `deny_unknown_fields`) for create, update, getById, list (pagination/search/filter), and delete
3. Create response DTO structs
4. Create a service struct with full CRUD (create, find_by_id, list, update, soft_delete)
5. Create a controller with associated async functions returning `Result<Response, AppError>`
6. Create a routes struct and merge it in `routes/v1/v1_route.rs`
7. Add the service to `types/services.rs` and `plugins/services.rs`
8. Add service tests in `tests/services` and route tests in `tests/integration`
9. Run `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings` and `cargo test -p <crate>`

Follow existing patterns in the target service exactly.
