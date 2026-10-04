---
applyTo: "apps/backend/**/schemas/**/*.rs,apps/backend/**/dtos/**/*.rs,apps/frontend/**/*.tsx,apps/frontend/**/*.ts,apps/mobile/**/*.tsx"
description: "End-to-end validation chain — SQL migration drives the validator request struct, which drives the Zod client schema, which drives UI error behaviour"
---

# End-to-End Validation Chain

The SQL migrations (and the Rust models mirroring them) are the **single source of truth**. Every constraint defined there must propagate to the `validator` request struct, the Zod client schema, and the UI error response — in that order, with nothing invented or omitted at any layer.

## The chain

```
SQL migration (NOT NULL, VARCHAR(N), CHECK, UNIQUE)
    ↓  (required, length, format implied by column name)
Rust request struct (serde + validator derives)
    ↓  on validation failure
400 Bad Request  { isSuccessful: false, message, errors: [{ field, message }] }
    ↓
frontend useMutation onError
    ↓
toast.error(message)

Meanwhile, client-side:
Zod schema (mirrors the request struct)
    ↓  on validation failure
react-hook-form field error
    ↓
inline error text below the field  ← NO toast here
```

## Error behaviour rules

| Source | UI behaviour |
|---|---|
| Zod client validation failure | Inline error below the field. No toast. |
| Server 400 (validator / business rule) | Toast with the server message. No inline error. |
| Server 409 (unique constraint) | Toast with a specific conflict message. |
| Server 500 | Toast: "Something went wrong. Please try again." |
| Network error | Toast: "Network error. Please check your connection." |

## SQL → validator → Zod mapping table

| SQL column | Rust request struct | Zod (frontend) |
|---|---|---|
| `VARCHAR NOT NULL` | `String` with `#[validate(length(min = 1))]` | `z.string().min(1, 'Required')` |
| nullable column | `Option<String>` | `z.string().optional()` |
| `VARCHAR(N)` | `#[validate(length(max = N))]` | `.max(N, 'Too long')` |
| email column name | `#[validate(email)]` | `z.string().email('Invalid email address')` |
| phone column name | `#[validate(custom(function = "valid_phone"))]` implementing `^(\+27\|0)[6-8][0-9]{8}$` | `z.string().regex(/^(\+27\|0)[6-8][0-9]{8}$/, 'Invalid phone number')` |
| `TIMESTAMPTZ` (ISO on the wire, dd/MM/yyyy on screen) | `DateTime<Utc>` (serde parses ISO 8601) or `NaiveDate` for date-only | `z.string().datetime()`, or `z.string().regex(/^\d{2}\/\d{2}\/\d{4}$/, 'Use dd/mm/yyyy')` for a free-text dd/mm/yyyy field — see `date-handling.instructions.md` |
| `NUMERIC(10,2)` | `Decimal` with a custom `non_negative` validator | `z.number().min(0, 'Must be 0 or greater')` |
| enum / lookup values | Rust `enum` with `#[serde(rename = "...")]` | `z.enum(['A', 'B'])` |
| `BOOLEAN NOT NULL` | `bool` | `z.boolean()` |
| `INTEGER` | `i32` with `#[validate(range(min = 0))]` | `z.number().int().min(0)` |
| UUID FK | `Uuid` (serde rejects malformed ids) | `z.string().uuid('Invalid selection')` |
| URL field | `#[validate(url)]` | `z.string().url('Invalid URL')` |
| `UNIQUE` | No validator rule (DB enforces) | No Zod rule | → returns 409 Conflict |
| minimum length implied | `#[validate(length(min = 2))]` (use judgment) | `.min(2, 'Too short')` |

## Backend — validator request struct and shared error formatting

Every request body, query string and path parameter goes through an extractor in `common/http`: `ValidatedJson<T>`, `ApiQuery<T>`, `ApiPath<T>`. They deserialise with `serde_path_to_error` (so a type mismatch names the exact field), run `validate()`, and turn failures into the field-level 400 envelope. Nothing is registered per service — using the extractor is enough.

```rust
#[derive(Debug, Clone, Deserialize, Validate)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct CreateUserRequest {
    #[validate(email, length(max = 255))]
    pub email: String,
    #[validate(length(min = 1, max = 100, message = "Name is required"))]
    pub name: String,
    pub role: RoleName,
}
```

Default messages (from `common/http/src/validation.rs`), used when no `message =` is given:

| Rule | Message |
|---|---|
| `email` | `Must be a valid email address` |
| `url` | `Must be a valid URI` |
| `length` | `Must be at least N characters` / `Must be at most N characters` / `Must be exactly N characters` |
| `range` | `Must be between A and B` / `Must be greater than or equal to A` |
| missing field | `Is required` |
| unknown field | `Unexpected property` |
| `regex` | `Invalid format` |

Field names in `errors[].field` are camelCase to match the JSON the client sent (`firstName`, not `first_name`); nested paths use dots and list indexes (`items[0].name`).

Rules that are not built in (phone numbers, known event names, non-negative decimals) are custom validator functions that return `Result<(), ValidationError>`, defined in their own file under `schemas/` and attached with `#[validate(custom(function = "valid_phone"))]`. They avoid a `regex` dependency and the lint-denied `unwrap` that compiling a pattern would need:

```rust
pub fn valid_phone(value: &str) -> Result<(), ValidationError> {
    let national = value
        .strip_prefix("+27")
        .map_or_else(|| value.to_owned(), |rest| format!("0{rest}"));
    let valid = national.len() == 10
        && national.starts_with('0')
        && matches!(national.as_bytes().get(1), Some(b'6'..=b'8'))
        && national.bytes().all(|byte| byte.is_ascii_digit());
    if valid {
        Ok(())
    } else {
        Err(ValidationError::new("phone").with_message("Invalid phone number".into()))
    }
}
```

Handlers never catch validation errors; they return `Result<Response, AppError>` and `AppError::Validation` renders the 400:

```rust
pub async fn create(
    State(state): State<AppState>,
    user: AuthUser,
    ValidatedJson(request): ValidatedJson<CreateUserRequest>,
) -> Result<Response, AppError> {
    let created = state.services.user.create(&request, user.id).await?;
    Ok((StatusCode::CREATED, Json(ApiResponse::success(created))).into_response())
}
```

Unique constraint violations return 409, mapped in the service from the repository's `DatabaseError`:

```rust
.map_err(|error| {
    if error.is_unique_violation() {
        AppError::Conflict("An account with this email already exists".to_owned())
    } else {
        AppError::internal(error)
    }
})
```

## Worked example — User entity

### 1. SQL migration

```sql
CREATE TABLE users (
    id        UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email     VARCHAR(255) NOT NULL UNIQUE,
    name      VARCHAR(100) NOT NULL,
    phone     VARCHAR(20),
    role_id   UUID NOT NULL REFERENCES roles (id),
    is_active BOOLEAN NOT NULL DEFAULT TRUE
);
```

### 2. Request struct (backend)

```rust
#[derive(Debug, Clone, Deserialize, Validate)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct CreateUserRequest {
    #[validate(email, length(max = 255))]
    pub email: String,
    #[validate(length(min = 1, max = 100, message = "Name is required"))]
    pub name: String,
    #[validate(length(max = 20), custom(function = "valid_phone"))]
    pub phone: Option<String>,
    pub role: RoleName,
}
```

### 3. Zod schema (frontend — mirrors the request struct exactly)

```typescript
export const createUserSchema = z.object({
  email: z.string().min(1, 'Required').email('Invalid email address').max(255, 'Too long'),
  name:  z.string().min(1, 'Name is required').max(100, 'Too long'),
  phone: z.string().regex(/^(\+27|0)[6-8][0-9]{8}$/, 'Invalid phone number').optional().or(z.literal('')),
  role:  z.enum(['Super Admin', 'Moderator', 'Support', 'Chat User'], { message: 'Please select a role' }),
});

export type CreateUserFormData = z.infer<typeof createUserSchema>;
```

### 4. React form (admin-web)

```typescript
import { useForm } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import { toast } from 'sonner';

export function CreateUserForm() {
  const queryClient = useQueryClient();

  const { register, handleSubmit, formState: { errors, isSubmitting } } = useForm<CreateUserFormData>({
    resolver: zodResolver(createUserSchema),
  });

  const mutation = useMutation({
    mutationFn: (data: CreateUserFormData) => userService.create(data),
    onSuccess: () => {
      toast.success('User created successfully');
      queryClient.invalidateQueries({ queryKey: ['users'] });
    },
    onError: (error: AxiosError<ApiErrorResponse>) => {
      const status = error.response?.status;
      const message = error.response?.data?.message;
      if (status === 409) {
        toast.error('An account with this email already exists');
      } else if (status === 400) {
        toast.error(message ?? 'Please check your input and try again');
      } else {
        toast.error('Something went wrong. Please try again.');
      }
    },
  });

  return (
    <form onSubmit={handleSubmit((data) => mutation.mutate(data))}>
      <div>
        <input type="email" {...register('email')} placeholder="Email address" />
        {errors.email && (
          <span className="text-destructive text-sm mt-1 block">{errors.email.message}</span>
        )}
      </div>

      <div>
        <input {...register('name')} placeholder="Full name" />
        {errors.name && (
          <span className="text-destructive text-sm mt-1 block">{errors.name.message}</span>
        )}
      </div>

      <div>
        <input {...register('phone')} placeholder="Phone (optional)" />
        {errors.phone && (
          <span className="text-destructive text-sm mt-1 block">{errors.phone.message}</span>
        )}
      </div>

      <div>
        <select {...register('role')}>
          <option value="">Select role</option>
          <option value="Super Admin">Super Admin</option>
          <option value="MODERATOR">Moderator</option>
          <option value="SUPPORT">Support</option>
          <option value="CHAT_USER">User</option>
        </select>
        {errors.role && (
          <span className="text-destructive text-sm mt-1 block">{errors.role.message}</span>
        )}
      </div>

      <button type="submit" disabled={isSubmitting || mutation.isPending}>
        {mutation.isPending ? 'Creating...' : 'Create User'}
      </button>
    </form>
  );
}
```

## Email — reject disposable/throwaway domains (service layer, after validation)

`#[validate(email)]` (Rust) and `.email()` (Zod) only validate syntax — they accept `anything@yopmail.com` just as happily as a real address. Domain-reputation is a business rule, not a shape constraint, so it is checked the same way a uniqueness constraint is: in the service layer, after validation passes, before the write.

```rust
use ru5ty_gate_types::is_disposable_email;

if is_disposable_email(&request.email) {
    return Ok(ApiResponse::failure("Please use a permanent email address"));
}
```

`is_disposable_email` checks the `DISPOSABLE_EMAIL_DOMAINS` slice in `common/types/src/disposable_email_domains.rs` — extend that list as new throwaway providers show up; it is a plain constant, so there is no dependency to install. `customer-api` additionally applies the larger `src/data/prohibited-email-domains.json` list through `is_email_domain_allowed` during registration. Both are case-insensitive. A match returns a `400` through the failure envelope, the same status as any other shape failure, with a field-less message (surfaces as a toast per the error-behaviour rules above, not an inline field error, since it is a server-side business rule rather than a client-side shape check).

Note: the seed list includes `proton.me`/`protonmail.com` alongside actual disposable-inbox services (`yopmail.*`, `mailinator.com`, `guerrillamail.*`, etc.) per explicit product decision — ProtonMail is a real, permanent mailbox provider, not a throwaway service, so this blocks legitimate privacy-conscious signups along with the intended throwaway ones. Revisit this specific entry if that tradeoff turns out to cost more signups than it prevents abuse.

## Rules — always enforced

- The Zod schema is derived FROM the Rust request struct. Never write one without updating the other.
- Never skip `length(min = 1)` on a required string field — empty strings deserialise successfully without it.
- `#[serde(deny_unknown_fields)]` on every request body — no exceptions.
- Always give custom messages to rules whose default text would confuse a user.
- Client-side Zod failure → inline error, no toast.
- Server 400/409/500 → toast, no inline error.
- The `errors` array in the 400 response always includes the `field` name so field mapping stays possible.
- Every form submit button shows a loading/pending state during mutation.
- `UNIQUE` constraint violations always return 409, not 400.
- Monetary `NUMERIC` fields use `z.number().min(0)` client-side; send as a number; the service parses into `Decimal` before the SQL write.
