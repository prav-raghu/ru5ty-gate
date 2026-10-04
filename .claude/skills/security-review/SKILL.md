---
description: Security audit for backend services and API endpoints — authentication gaps, injection risks, hardcoded secrets, and permission bypass vectors.
disable-model-invocation: true
argument-hint: <branch, path, or service name, e.g. "apps/backend/customer-api/src/routes">
---

# Security Review

Audit the changes or files at: $ARGUMENTS

Run this diff first if reviewing a branch:

!`git diff origin/main..HEAD -- $ARGUMENTS 2>/dev/null || find $ARGUMENTS -name "*.rs" -o -name "*.ts" | head -20`

## Audit checklist

### Authentication & Authorization

- [ ] Every service validates JWTs itself in `authenticate` (signature, type, scope, blacklist, `minIat`); the gateway never decodes or trusts a token
- [ ] No route skips auth via query param, header trick, or env flag
- [ ] `public()` routers contain only genuinely public endpoints
- [ ] `require_permission` is applied to every protected route group
- [ ] `AuthUser` comes only from the `authenticate` layer, never from the request body or query string

### Input Validation

- [ ] Every request body goes through `ValidatedJson` (or `ApiQuery`/`ApiPath`) with `validator` rules
- [ ] `#[serde(deny_unknown_fields)]` on all request structs
- [ ] No SQL injection vectors (every value bound; dynamic filters use `QueryBuilder::push_bind`)
- [ ] File uploads validate MIME type and size before processing

### Secrets & Credentials

- [ ] No hardcoded secrets, API keys, tokens, or connection strings anywhere
- [ ] No sensitive values in log output (check `SENSITIVE_KEYS` list is complete)
- [ ] `.env` is gitignored; only `.env.example` with placeholders is committed
- [ ] Passwords stored as bcrypt hashes, never plaintext

### Output & Data Exposure

- [ ] Queries list explicit columns; response DTOs never carry password hashes or TOTP secrets
- [ ] Audit log `redact()` covers password, token, secret, hash, two_factor_secret
- [ ] Webhook payloads contain no PII, passwords, or internal system IDs
- [ ] No stack traces or internal error messages in production API responses

### Infrastructure

- [ ] No wildcard `*` in CORS `origin` config in production
- [ ] `security_headers` layer is registered on every Rust service
- [ ] Rate limiting is applied per policy (global 200/min, auth 10/min, sensitive 5/min)
- [ ] SSRF prevention (`TargetPolicy::PublicOnly`, no redirects) in place on any service making outbound HTTP calls to user-supplied URLs

## Report format

Group findings as:

**Blockers** (must fix before merge) → **Warnings** (should fix) → **Suggestions** (nice to have)

Each finding: location, issue, concrete fix.
