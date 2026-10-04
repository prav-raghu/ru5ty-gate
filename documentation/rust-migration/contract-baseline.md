# Backend Contract Baseline

This is the wire contract the Rust services must preserve. Frontends, mobile and n8n depend only on this surface. Every route below is mounted under `/api/v1` (a `/api/v2` prefix also exists and exposes only `GET /health` returning `{ "status": "healthy", "version": "v2" }`).

## Cross-cutting behaviour

| Concern | Contract |
|---|---|
| Response envelope | `{ isSuccessful: boolean, data?: T, message?: string, errors?: { field: string, message: string }[] }` |
| Validation failure | 400 with `message: "Validation failed"` and field-level `errors` |
| Status codes | 201 create; 200 read/update/delete; 400 validation or business rule; 401 unauthenticated; 403 forbidden; 404 not found; 409 duplicate; 500 unexpected |
| Request bodies | Unknown properties rejected; required strings have `minLength: 1`; emails and UUIDs format-checked |
| Dates | ISO 8601 in both directions |
| Public routes | Flagged `isPublic: true`; everything else requires a valid access token |
| Permissions | Admin batch and reporting routes require a permission from `common/types/src/permissions.ts` |
| Rate limits | Public 30/min per IP; authenticated 120/min per user; admin 300/min per user; named tiers `auth`, `sensitiveEndpoints`, `adminOperations` per route |
| Headers | Helmet defaults; CORS with credentials and a single `CORS_ORIGIN` |
| Request logging | Health and readiness routes excluded |
| Docs | Swagger UI at `/docs` outside production |

## customer-api (port 4002)

| Method | Path | Public | Notes |
|---|---|---|---|
| GET | /ping | yes | liveness |
| GET | /ready | yes | database and Redis check |
| POST | /auth/register | yes | tier `auth` |
| POST | /auth/login | yes | tier `auth` |
| POST | /auth/refresh | yes | tier `auth`, refresh rotation |
| POST | /auth/logout | no | invalidates every session for the user |
| GET | /auth/verify/:token | yes | email verification |
| POST | /auth/resend-verification-email | yes | tier `sensitiveEndpoints` |
| GET | /users | no | paginated list |
| GET | /users/:userId | no | |
| GET | /users/export | no | file export |
| GET | /users/export/stream | no | streamed export |
| POST | /webhooks/subscriptions | no | |
| GET | /webhooks/subscriptions | no | |
| GET | /webhooks/subscriptions/:id | no | |
| PUT | /webhooks/subscriptions/:id | no | |
| DELETE | /webhooks/subscriptions/:id | no | |
| GET | /webhooks/subscriptions/:id/deliveries | no | |
| POST | /webhooks/subscriptions/:id/regenerate-secret | no | |
| POST | /webhooks/retry | no | |

## admin-api (port 4001)

| Method | Path | Public | Notes |
|---|---|---|---|
| GET | /ping | yes | liveness |
| GET | /ready | yes | database and Redis check |
| POST | /auth/login | yes | returns `mfaToken` when two-factor is enabled |
| POST | /auth/verify-login-mfa | yes | issues real tokens after TOTP passes |
| POST | /auth/refresh | yes | |
| POST | /auth/forgot-password | yes | |
| POST | /auth/reset-password | yes | |
| GET | /auth/logout | no | |
| GET | /auth/me | no | |
| GET | /users/roles | no | |
| GET | /users/statuses | no | |
| POST | /users/onboarding | no | |
| POST | /users/resend-verification | no | |
| GET | /users/check-email/:email | no | tier `sensitiveEndpoints` |
| GET | /users/check-username/:username | no | tier `sensitiveEndpoints` |
| PUT | /users/profile | no | |
| POST | /users/change-password | no | tier `sensitiveEndpoints` |
| POST | /users/2fa/setup | no | tier `sensitiveEndpoints` |
| POST | /users/2fa/verify | no | tier `sensitiveEndpoints` |
| POST | /users/2fa/disable | no | tier `sensitiveEndpoints` |
| GET | /users/:userId/details | no | tier `adminOperations` |
| POST | /batch/users/create | no | permission `BATCH_WRITE` |
| POST | /batch/users/update-status | no | permission `BATCH_WRITE` |
| POST | /batch/users/delete | no | permission `BATCH_WRITE` |
| POST | /batch/custom | no | permission `BATCH_WRITE` |
| POST | /reports/generate | no | permission `REPORT_EXPORT` |
| GET | /reports/stream | no | permission `REPORT_EXPORT` |
| GET | /reports/user-activity | no | permission `REPORT_VIEW` |
| GET | /reports/webhook-delivery | no | permission `REPORT_VIEW` |
| GET | /reports/system-metrics | no | permission `REPORT_VIEW` |

The one-time `/auth/bootstrap-admin` route described in `CLAUDE.md` is not present in the Node code today. It is carried into the Rust admin-api as a new route in the admin-api phase.

## schedule-api (port 4003)

| Method | Path | Public | Notes |
|---|---|---|---|
| GET | /ping | yes | liveness |
| GET | /ready | yes | database and Redis check |

Background behaviour: a cron scheduler runs the webhook processor job. Protected routes use an API-key guard rather than a user JWT.

## api-gateway (port 4000)

| Surface | Behaviour |
|---|---|
| `/api/*` | Proxied to customer-api (64 to 128 upstream connections) |
| `/admin/*` | Proxied to admin-api |
| `/scheduler/*` | Proxied to schedule-api |
| `/graphql` | Apollo-style GraphQL with a user schema |
| `/docs` | Swagger UI |
| Service health | Aggregated downstream health routes |
| Metrics | Prometheus metrics with `gateway_` prefix |

## Plugin order to preserve

Error handler, CORS, helmet, rate limit, request logger, API version, swagger, database, services, response timestamp (admin-api and schedule-api only), auth guard, then routes.

## How parity is verified

1. Each Rust service ships an OpenAPI document generated from its handlers.
2. A contract test per route asserts status code, envelope shape and field-level validation errors against the tables above.
3. A Node service is removed only when its table is fully covered.
