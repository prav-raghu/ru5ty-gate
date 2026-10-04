---
applyTo: "apps/backend/**/application.rs,apps/backend/**/schemas/**/*.rs,apps/backend/**/dtos/**/*.rs,apps/backend/**/controllers/**/*.rs"
description: "OpenAPI/Swagger generation for the Rust services with utoipa — convention to follow when API documentation is added"
---

Status: the Node services exposed Swagger UI generated from AJV schemas. The Rust services do not expose OpenAPI documentation yet. This file records the convention to follow so the documentation lands consistently across services when it is added. Until then, the request structs in `schemas/` and the response structs in `dtos/` are the contract, and `documentation/rust-migration/contract-baseline.md` lists every route.

## Required Crates

Add to each backend service's `Cargo.toml`:

```toml
utoipa = { version = "5", features = ["axum_extras", "chrono", "uuid", "decimal"] }
utoipa-swagger-ui = { version = "9", features = ["axum"] }
```

Run `cargo deny check` after adding them.

## Annotating Types

Request and response structs derive `ToSchema`; query and path structs derive `IntoParams`:

```rust
#[derive(Debug, Clone, Deserialize, Validate, ToSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct CreateProductRequest {
    #[schema(min_length = 2, max_length = 200)]
    #[validate(length(min = 2, max = 200))]
    pub name: String,
}
```

Keep the `#[schema(...)]` constraints identical to the `#[validate(...)]` rules — the documented contract must match what is enforced.

## Annotating Handlers

```rust
#[utoipa::path(
    get,
    path = "/api/v1/products/{productId}",
    tag = "Products",
    params(ProductPath),
    responses(
        (status = 200, description = "Product found", body = ApiResponse<ProductDto>),
        (status = 404, description = "Product not found", body = ApiResponse<()>),
    ),
    security(("bearerAuth" = []))
)]
pub async fn get(...)
```

Public handlers use `security(())` to opt out of the global bearer requirement. Every handler gets a `tag` — untagged routes land under a generic "default" group.

## Registration

A single `ApiDoc` struct per service in `src/plugins/openapi.rs`:

```rust
#[derive(OpenApi)]
#[openapi(
    paths(ProductController::get, ProductController::create),
    components(schemas(ProductDto, CreateProductRequest, FieldError)),
    modifiers(&BearerAuth),
    tags((name = "Products"))
)]
pub struct ApiDoc;
```

`BearerAuth` is a `Modify` implementation that adds the `bearerAuth` HTTP bearer JWT security scheme. Mount the UI in `application.rs` only when `!config.production`:

```rust
let router = if config.production {
    router
} else {
    router.merge(SwaggerUi::new("/docs").url("/docs/openapi.json", ApiDoc::openapi()))
};
```

## Logging the docs link

Build the service's `ServerInfo` with `.with_docs("/docs")` in `Application::start`:

```rust
let info = ServerInfo::new("admin-api", env!("CARGO_PKG_VERSION"), config.port, config.production)
    .with_docs("/docs");
serve(self.router(), &info).await
```

In development `serve` then logs `API docs: http://localhost:4001/docs` under the startup banner. `ServerInfo::docs_url()` returns `None` when `production` is true, so the link is never logged in production. Only call `.with_docs` once Swagger UI is actually mounted.

## Accessing Docs

| Environment | URL |
|-------------|-----|
| Local dev | `http://localhost:{PORT}/docs` |
| Staging | `http://staging-host:{PORT}/docs` |
| Production | **not exposed** (`APP_ENV=production` disables `/docs`) |

| Service | Port | Docs URL |
|---------|------|---------|
| api-gateway | 4000 | `http://localhost:4000/docs` |
| admin-api | 4001 | `http://localhost:4001/docs` |
| customer-api | 4002 | `http://localhost:4002/docs` |
| schedule-api | 4003 | `http://localhost:4003/docs` |

The request logger already skips paths starting with `/docs`.

## Rules

- NEVER expose `/docs` in production — controlled by `config.production`
- ALWAYS add `tag` to every documented handler
- ALWAYS document `200`/`201` and the `400`, `401`, `404`, `409` shapes for frequently-used endpoints
- Keep one `ApiDoc` per service; add new handlers to its `paths(...)` list in the same change that creates them
- A test should fetch `/docs/openapi.json` in development mode and assert that every route registered in `V1Routes` appears, so the documentation cannot drift
