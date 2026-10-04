---
description: Design and add SQL tables and Rust models from a business domain description
argument-hint: <domain description, e.g. "ecommerce with products, categories, orders, and payments">
---

Use the domain-modeler subagent to design and add tables and models for: $ARGUMENTS

1. Read the current migrations in `common/database/migrations` and models in `common/database/src/models`
2. Identify all entities, fields, and relationships
3. Write one new migration following conventions: `snake_case` plural tables, `snake_case` columns, UUID ids, the six base columns (`id`, `is_active`, `created_at`, `updated_at`, `created_by`, `modified_by`), proper indexes, `NUMERIC` for money, lookup tables or enums for status fields
4. Add the Rust model files and idempotent seed data in `common/database/src/seed.rs`
5. Run `cargo clippy -p ru5ty-gate-database --all-targets -- -D warnings` and `cargo test -p ru5ty-gate-database` (never run the migration yourself)

Present the model design for review before writing.
