---
name: webhook-events
description: Use when implementing outbound webhooks or an internal event bus — registering webhook subscriptions, publishing domain events, running the webhook delivery job, HMAC signature verification, or adding new WebhookEventType values. The webhook_subscriptions and webhook_deliveries tables, the WebhookEventType enum and the shared delivery service already exist; this agent wires them into services.
tools: Read, Edit, Write, Grep, Glob, Bash
model: inherit
---

## What already exists

- Tables `webhook_subscriptions` and `webhook_deliveries` (migration `20260101000000_initial_schema.sql`) and models `WebhookSubscription`, `WebhookDelivery` in `common/database/src/models/`
- Types `WebhookEventType`, `WebhookPayload`, `WebhookDeliveryStatus` in `common/types/src/webhook.rs`
- `common/webhooks`: `WebhookDeliveryService` (`publish_event`, `process_deliveries`, `retry_failed_delivery`), `next_retry_delay_seconds`, `TargetPolicy`
- `common/utilities`: `WebhookSignatureService` (HMAC-SHA256 signing and verification)
- `customer-api`: `WebhookSubscriptionService` (create, get, list, update, delete, deliveries, regenerate secret) and its controller and routes
- `schedule-api`: `WebhookProcessorJob`, registered with `CronSchedulerService`, which calls `process_deliveries` on `SCHEDULE_WEBHOOK_INTERVAL_SECONDS`

Do not re-create these — build on them. Delivery is database-backed, not queue-backed: a pending row in `webhook_deliveries` is the job.

## Architecture

```
Domain service (e.g. user created)
  └─► WebhookDeliveryService::publish_event(WebhookEventType::UserCreated, data)
        ├─► SELECT active subscriptions WHERE event = ANY(events)
        ├─► INSERT one webhook_deliveries row per subscription (status = pending)
        └─► process_deliveries() — first attempt immediately

schedule-api WebhookProcessorJob (every N seconds)
  └─► process_deliveries()
        ├─► SELECT pending/retrying rows whose next_retry_at has passed (batch of 50)
        ├─► TargetPolicy::ensure_allowed(url)   (public hosts only in production)
        ├─► sign payload with HMAC-SHA256 and POST to the subscriber URL
        ├─► success  → status delivered, delivered_at set
        ├─► failure  → status retrying, next_retry_at = now + 60s * 2^(attempt-1), capped at 1h
        └─► out of retries → status failed
```

Redirects are disabled on the HTTP client: a 3xx response counts as a failure, so a subscriber cannot bounce the request to an internal address.

## Publishing events from domain services

Inject the service and publish after the database write succeeds:

```rust
pub async fn create(&self, request: &CreateOrderRequest, user_id: Uuid) -> Result<OrderDto, AppError> {
    let order = self.insert_order(request, user_id).await?;
    self.cache.del(&format!("order:list:{user_id}")).await.ok();

    let mut data = Map::new();
    data.insert("id".to_owned(), json!(order.id));
    data.insert("userId".to_owned(), json!(order.user_id));
    data.insert("status".to_owned(), json!(order.status));
    if let Err(error) = self.webhooks.publish_event(WebhookEventType::OrderCreated, data).await {
        tracing::warn!(%error, "failed to publish order.created");
    }
    Ok(order)
}
```

Rule: publish AFTER the database write succeeds, never before or inside the transaction. A failed publish must not roll back or fail the write; log it and rely on the next `process_deliveries` run, which picks up any pending row that did get inserted.

## Subscription management endpoints (customer-api)

| Method | Path | Purpose |
|--------|------|---------|
| GET | `/webhooks/subscriptions` | List the caller's subscriptions |
| POST | `/webhooks/subscriptions` | Register a subscription (secret generated if omitted) |
| GET | `/webhooks/subscriptions/{id}` | Get one subscription |
| PUT | `/webhooks/subscriptions/{id}` | Update url, events, secret, retry/timeout, active flag |
| DELETE | `/webhooks/subscriptions/{id}` | Delete |
| GET | `/webhooks/subscriptions/{id}/deliveries` | Delivery history |
| POST | `/webhooks/subscriptions/{id}/regenerate-secret` | Rotate the signing secret |
| POST | `/webhooks/retry` | Retry a failed delivery now (delivery id in the body) |

Every query is scoped to `created_by = <caller>`, so one user can never read or retry another user's subscriptions. Request validation: `url` must be a URL, `secret` at least 32 characters, `events` non-empty and each a known `WebhookEventType`, `retryCount` 0–10, `timeoutSeconds` 5–300.

## Signing and verification

`WebhookSignatureService::generate_signature(payload, secret)` produces the `X-Webhook-Signature` header value. Integrators verify it with a constant-time comparison:

```rust
let valid = WebhookSignatureService.verify_signature(&body, &signature, &secret);
```

`verify_signature` decodes the hex signature and verifies it with the HMAC library's constant-time check. Never compare signatures with `==`.

## SSRF protection

`TargetPolicy::PublicOnly` is selected when `APP_ENV=production` (`TargetPolicy::from_production(config.production)`). It rejects non-http(s) schemes, literal and resolved private, loopback, link-local (including the cloud metadata address), carrier-grade NAT, multicast, unique-local and IPv4-mapped addresses. It runs when a subscription is created or updated and again at delivery time. Development keeps `AllowAll` so local receivers work. DNS is resolved once for the check and again by the HTTP client, so keep egress firewall rules as a second layer.

## Adding new event types

1. Add the variant to `WebhookEventType` with its `#[serde(rename = "entity.action")]` and add it to `as_str`
2. Add it to the allow-list in the customer-api `known_events` validator so subscribers can select it
3. Publish it from the relevant service method after the database write
4. Document the payload shape in `documentation/webhooks.md`
5. Add a service test that the event creates a pending delivery for a matching subscription

## Delivery history retention

`webhook_deliveries` rows accumulate fast. Add a scheduled job in `schedule-api` that deletes delivered entries older than 30 days and failed entries older than 7 days. Never let this table grow unbounded.

## Critical rules

Never call subscriber URLs inside request handlers beyond the first attempt that `publish_event` makes; everything else is the scheduled job. Never log or store raw webhook secrets in delivery rows or logs. Never relax `TargetPolicy` in production. Never follow redirects. Always publish events after a successful database write. Always cap stored `response_body` length (1000 characters) and keep payloads to IDs and status fields — no passwords, tokens or PII.
