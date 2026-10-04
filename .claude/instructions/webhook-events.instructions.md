---
applyTo: "apps/backend/**/services/**,common/webhooks/src/**"
description: "Outbound webhook and event conventions - always database-backed, always HMAC-signed, always post-write"
---

When publishing domain events or delivering webhooks:

## Publish After Write, Never Before

```rust
let entity = self.insert(request).await?;
if let Err(error) = self.webhooks.publish_event(WebhookEventType::UserCreated, data).await {
    tracing::warn!(%error, "failed to publish user.created");
}
```

If publishing fails (database hiccup), the original write stands and the request still succeeds. Pending rows are retried by the scheduled job. Never wrap publish in the same transaction as the write.

## Which Services Publish Events

Publish only from services that own state changes external systems might care about:

- `OrderService` → `OrderCreated`, `OrderUpdated`, `OrderCompleted`
- `UserService` → `UserCreated`, `UserUpdated`, `UserDeleted`
- `PaymentService` → `PaymentSuccess`, `PaymentFailed`

Read-only services (search, reporting) never publish events.

## Event Payload Shape

Keep payloads minimal — IDs and status fields only. Subscribers fetch full data from your API if they need it:

```rust
let mut data = Map::new();
data.insert("id".to_owned(), json!(order.id));
data.insert("userId".to_owned(), json!(order.user_id));
data.insert("status".to_owned(), json!(order.status));
```

NEVER include passwords, tokens, PII fields, or internal system IDs in webhook payloads.

## Delivery Location

Delivery is performed by the shared `WebhookDeliveryService` in `common/webhooks`. The recurring delivery job (`WebhookProcessorJob`) lives in `apps/backend/schedule-api` — not in customer-api or admin-api. Register additional jobs in `schedule-api`'s `plugins/services.rs`.

## SSRF Prevention

`TargetPolicy::PublicOnly` (selected by `APP_ENV=production`) validates every subscriber URL when it is saved and again before each delivery. It rejects non-http(s) schemes and private, loopback, link-local, carrier-grade NAT, multicast and unique-local addresses, including hostnames that resolve to them. The HTTP client never follows redirects.

## Delivery History Retention

`webhook_deliveries` rows accumulate fast. Add a scheduled cleanup job in `schedule-api` that deletes delivered entries older than 30 days and failed entries older than 7 days. Never let this table grow unbounded.
