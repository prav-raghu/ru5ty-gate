---
applyTo: "apps/**/*.ts,apps/**/*.tsx,apps/backend/**/*.rs,common/**/*.rs"
description: "Date/time convention across DB, service/API, and UI layers — one storage format, one wire format, one display format"
---

# Date & Time Handling — DB → Service → UI

Three layers, three different jobs, three different formats. Never let a display format leak into storage or the wire, and never let storage format leak into the UI.

## 1. Database layer — `TIMESTAMPTZ`, stored UTC

Every timestamp column is Postgres `TIMESTAMPTZ`, always written and read in UTC, and mapped to `chrono::DateTime<Utc>` in Rust models. Never store a formatted string (`"25/12/2025"`) in a date column, and never add a parallel `VARCHAR` column to hold a display-formatted copy of a date — format at render time, not at rest.

```sql
created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
event_date TIMESTAMPTZ NOT NULL
```

`TIMESTAMPTZ` normalises to UTC internally regardless of session timezone — this is already correct by default and needs no extra configuration. Use `NaiveDate` / `DATE` only for genuine date-only business values (a birthdate), and bind them explicitly.

## 2. Service / API layer — ISO 8601, always, both directions

- **Outbound** (API response): a `DateTime<Utc>` field serialises to ISO 8601 UTC through serde (`2025-12-25T14:30:00Z`). Never format a date by hand before sending it — that is a UI concern, not a service concern. Response timestamps stamped by middleware (`dateTimeStamp`) use `DateUtil` and are ISO 8601 too.
- **Inbound** (request body): declare the field as `DateTime<Utc>` (full timestamp) or `NaiveDate` (date only) on the request struct. serde rejects anything that is not ISO 8601 and `ValidatedJson` reports it as a field-level 400:

```rust
pub event_date: DateTime<Utc>,
pub birth_date: Option<NaiveDate>,
```

- The service binds the typed value directly to the query (`.bind(request.event_date)`) — never accept a `String` and parse it later in the service.
- Never accept `dd/mm/yyyy` on a backend request body. The `dd/mm/yyyy [HH:mm:ss]` format is a UI presentation concern only — the frontend converts it to ISO 8601 before the request ever leaves the browser (see §3). If a backend struct is parsing `dd/mm/yyyy`, that is a bug — fix the frontend's outbound mapping instead of relaxing the type.
- `DateUtil` in `ru5ty-gate-utilities` covers ISO parsing, validation and "now" helpers for the cases where a string must be handled (query parameters, CSV imports).

## 3. UI layer — display and input as `dd/MM/yyyy`, optional `HH:mm:ss`

Both frontend apps already depend on `date-fns` — use it, not `Date.prototype.toLocaleDateString()` (locale-dependent, not guaranteed `dd/mm/yyyy` across browsers/OS locales) and not a second date library.

```typescript
// src/utilities/format-date.ts
import { format, parse, isValid } from 'date-fns';

const DATE_FORMAT = 'dd/MM/yyyy';
const DATE_TIME_FORMAT = 'dd/MM/yyyy HH:mm:ss';

export function formatDate(value: string | Date): string {
  const date = typeof value === 'string' ? new Date(value) : value;
  return format(date, DATE_FORMAT);
}

export function formatDateTime(value: string | Date): string {
  const date = typeof value === 'string' ? new Date(value) : value;
  return format(date, DATE_TIME_FORMAT);
}

export function parseDateInput(value: string, withTime = false): Date | null {
  const parsed = parse(value, withTime ? DATE_TIME_FORMAT : DATE_FORMAT, new Date());
  return isValid(parsed) ? parsed : null;
}

export function toApiDate(value: Date): string {
  return value.toISOString();
}
```

- **Display**: every place a date/timestamp is rendered — tables, detail views, PDFs, exports — goes through `formatDate`/`formatDateTime`. Time is appended (`HH:mm:ss`) only when the field is genuinely a timestamp the user cares about to the second (audit trails, activity logs); a plain business date (order date, birthdate) shows date-only.
- **Input**: a native date picker (`<input type="date">` wrapped by a component, or a calendar widget) already returns a real `Date`/ISO value — no parsing needed, and this is the default choice. Only when a field genuinely requires free-text entry does `parseDateInput` come into play, and that field's Zod schema validates the `dd/MM/yyyy` shape before parsing:

```typescript
date: z.string().regex(/^\d{2}\/\d{2}\/\d{4}$/, 'Use dd/mm/yyyy').refine((v) => parseDateInput(v) !== null, 'Invalid date')
```

- **Outbound**: before the value reaches `apiClient`, convert with `toApiDate` (or just send the picker's native ISO value straight through) — the wire format is always ISO 8601, never `dd/mm/yyyy`, matching §2.

## Summary

| Layer | Format | Never |
|---|---|---|
| Database | `TIMESTAMPTZ`, UTC (`DateTime<Utc>` in Rust) | A `VARCHAR` column holding a formatted date |
| Service / API (request + response) | ISO 8601 (`DateTime<Utc>` or `NaiveDate` via serde) | `dd/mm/yyyy` on the wire in either direction |
| UI display | `dd/MM/yyyy`, optional ` HH:mm:ss` via `date-fns` | `toLocaleDateString()`, a second date library, hand-rolled string splitting |
