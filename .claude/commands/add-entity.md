---
description: Add a new domain entity with full CRUD across database, backend API, and frontend
argument-hint: <entity name and fields, e.g. "product with name, price, category, description, image">
---

Use the full-stack-orchestrator subagent to add this domain entity end to end: $ARGUMENTS

1. SQL migration in `common/database/migrations` and the Rust model in `common/database/src/models` with proper types, relations, and indexes
2. Idempotent seed data for any lookup/reference tables
3. Backend API (request struct, DTO, service, controller, route) in the appropriate service
4. Frontend pages (list, detail, create form, edit form) in admin-web
5. React Query hooks and Zod validation schemas
6. Wire into existing route registration and the service container (`types/services.rs`, `plugins/services.rs`)

Present the plan and wait for confirmation before writing code. Do not run migrations — the developer does. Run `cargo clippy` and `cargo test` for the touched crates.
