# GraphQL Layer Documentation

## Overview

The API Gateway includes an **optional** GraphQL layer that provides a unified query interface over the backend REST services. It is implemented with `async-graphql` and `async-graphql-axum` inside `apps/backend/api-gateway` and calls the existing REST endpoints, so business logic stays in the services.

## Architecture

```
Client → api-gateway /graphql (async-graphql) → REST microservices
                                                  └── customer-api (users)
```

### Key Components

1. **GraphQL router** (`src/plugins/graphql.rs`)
   - `graphql_router(&ServiceConfig) -> Option<Router>`; returns `None` when `GRAPHQL_ENABLED` is not `true`
   - Mounted by `application.rs` next to the proxy routes

2. **Routes** (`src/routes/graphql_route.rs`)
   - `POST {GRAPHQL_PATH}` executes the request; the `Authorization: Bearer` token is extracted from the headers and added to the request data as `BearerToken`
   - `GET {GRAPHQL_PATH}` serves GraphiQL when the playground is enabled

3. **Schema builder** (`src/graphql/schema_builder.rs`)
   - `build_schema(customer_api_url, introspection) -> GatewaySchema`
   - Applies `limit_depth(10)` and `limit_complexity(200)`, and disables introspection when it is off

4. **Types** (`src/graphql/schemas/`)
   - One type per file: `User`, `UserResponse`, `UsersResponse`, `CreateUserInput`, `UpdateUserInput`

5. **Resolvers** (`src/graphql/resolvers/`)
   - `QueryRoot` and `MutationRoot` `#[Object]` impls, one file each
   - `UserApi` is the REST client (`reqwest`) that forwards the bearer token and maps the REST envelope to `{ success, data, error }`

## Configuration

### Environment Variables

```env
GRAPHQL_ENABLED=true
GRAPHQL_PATH=/graphql
GRAPHQL_PLAYGROUND=true
GRAPHQL_INTROSPECTION=true

CUSTOMER_API_URL=http://localhost:4002
ADMIN_API_URL=http://localhost:4001
SCHEDULER_API_URL=http://localhost:4003
```

`GraphqlConfig::from_env(env, production)` forces `playground` and `introspection` to `false` when `APP_ENV=production`, whatever the variables say.

### Disabling GraphQL

Set `GRAPHQL_ENABLED=false` (or leave it unset) to disable the GraphQL layer entirely.

## Usage Examples

### GraphQL Endpoint

Access GraphQL at: `http://localhost:4000/graphql`

### Example Queries

#### Get User by ID

```graphql
query GetUser {
  getUser(id: "6b0f0a52-1d7a-4c4e-9d0c-2f4d8d3f9a11") {
    success
    data {
      id
      email
      firstName
      role
    }
    error
  }
}
```

#### Get All Users

```graphql
query GetAllUsers {
  getUsers {
    success
    data {
      id
      firstName
    }
    error
  }
}
```

### Example Mutations

```graphql
mutation UpdateUser {
  updateUser(id: "6b0f0a52-1d7a-4c4e-9d0c-2f4d8d3f9a11", input: { firstName: "Jane", isActive: true }) {
    success
    data {
      id
      firstName
      isActive
    }
    error
  }
}
```

The fields come from `src/graphql/schemas/`: `User` exposes `id`, `email`, `firstName`, `lastName`, `role`, `isActive`, `createdAt`, `updatedAt` (customer-api `username` maps to `firstName`). The resolvers translate each operation to the corresponding customer-api REST call.

## Authentication

GraphQL requests support JWT authentication via Bearer tokens:

```bash
curl -X POST http://localhost:4000/graphql \
  -H "Authorization: Bearer YOUR_JWT_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"query": "{ getUsers { success data { id firstName } } }"}'
```

The token is forwarded unchanged to the REST service, which performs the real authentication and authorisation. The gateway never trusts or decodes it.

## Frontend Integration

Any GraphQL client works. With Apollo Client:

```typescript
import { ApolloClient, InMemoryCache, createHttpLink } from "@apollo/client";
import { setContext } from "@apollo/client/link/context";

const httpLink = createHttpLink({ uri: "/graphql" });
const authLink = setContext((_, { headers }) => ({
  headers: { ...headers, authorization: accessToken ? `Bearer ${accessToken}` : "" },
}));

export const client = new ApolloClient({ link: authLink.concat(httpLink), cache: new InMemoryCache() });
```

With React Query, post the query string with the existing Axios client and unwrap `data`.

## Adding New Schemas

### 1. Create the type files

One file per type in `src/graphql/schemas/`, re-exported from `schemas/mod.rs`:

```rust
#[derive(SimpleObject, Clone, Debug)]
pub struct ProductType {
    pub id: ID,
    pub name: String,
}

#[derive(SimpleObject, Clone, Debug)]
pub struct ProductResponse {
    pub success: bool,
    pub data: Option<ProductType>,
    pub error: Option<String>,
}
```

Inputs derive `InputObject`.

### 2. Create the REST client and resolvers

Add `product_api.rs` beside `user_api.rs` with a `ProductApi { client, base_url }` that calls the service and maps the envelope. Add the operations to `QueryRoot`/`MutationRoot` (or, when a root grows large, split it into merged objects with `MergedObject`), reading the token with the shared `token(context)` helper.

### 3. Register in the schema builder

Pass the new API client into `build_schema` through `.data(Arc::new(ProductApi::new(...)))`. Keep the depth and complexity limits.

## Error Handling

- REST failures are mapped to `{ success: false, error: "<message>" }` in the response type, not to a GraphQL error
- Transport failures and unreadable bodies become a GraphQL error with `extensions.code = "INTERNAL_SERVER_ERROR"`
- Invalid queries (syntax, depth, complexity) return the standard `errors` array with HTTP 200, as GraphQL specifies

## Performance Considerations

- Resolvers call REST sequentially per field; avoid deep nesting that fans out many calls
- Depth is limited to 10 and complexity to 200; raise them deliberately
- Cache read-heavy REST responses in the owning service, not in the gateway

## Security

- Introspection and the playground are always off in production
- Depth and complexity limits protect against expensive queries
- The bearer token is only forwarded, never logged (request logging masks `authorization`)
- Rate limiting applies to `/graphql` through the gateway's global limiter (`RATE_LIMIT_MAX`)

## Testing

```bash
curl -X POST http://localhost:4000/graphql \
  -H "Content-Type: application/json" \
  -d '{"query": "{ __typename }"}'
```

Integration tests live in `apps/backend/api-gateway/tests/integration/graphql.rs` and exercise the schema against a stub REST server. In development, open `http://localhost:4000/graphql` in a browser for GraphiQL.

## Best Practices

1. Keep resolvers thin: map, forward the token, return
2. One type per file
3. Do not add business rules to the gateway
4. Add an integration test for every new operation
5. Document new operations in this file

## Troubleshooting

### GraphQL not available

- Check `GRAPHQL_ENABLED=true`
- Check the gateway logs for "GraphQL endpoint available"

### 401 Unauthorized

- The REST service rejected the forwarded token; verify the `Authorization: Bearer` header and token expiry

### Schema errors

- `cargo test -p ru5ty-gate-api-gateway` builds the schema and runs queries against it

### Connection errors

- Verify `CUSTOMER_API_URL` is reachable from the gateway container
