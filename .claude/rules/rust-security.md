---
paths:
  - "**/*.rs"
---

# Rust Security

## Secrets Management
- Load secrets from environment variables; never hardcode
- Fail fast at startup if required secrets are missing (`expect("VAR must be set")`)
- Use sealed types to prevent accidental logging of sensitive values:

```rust
pub struct Secret(String);

impl std::fmt::Debug for Secret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "***REDACTED***")
    }
}
```

## SQL Injection Prevention
- **Always** use `sqlx::query!` or `sqlx::query_as!` — never format user input into SQL strings
- Compile-time query verification is the rule, not the exception

## Input Validation
- Parse, don't validate — convert unstructured input to typed structs at the boundary
- Every endpoint validates all fields in a `validate()` method before touching DB, cache, or crypto
- Invalid input returns 422; validation errors must not leak internal details

## Unsafe Code
- Every `unsafe` block requires a `// SAFETY:` comment documenting the invariants held
- Treat any `unsafe` without a comment as a bug

## Cryptography (project-specific)
- Passwords: Argon2id only, m_cost=64MB (2026 standard)
- JWTs: Ed25519 (EdDSA) signatures only — no HS256
- Tokens: short-lived access tokens + long-lived refresh tokens stored in Redis

## Error Messages
- Never expose internal details (DB errors, stack traces) to API clients
- Log the full error server-side; return a generic message to the client:

```rust
Err(e) => {
    tracing::error!("db error: {e}");
    AppError::internal()
}
```

## Dependency Security
- Run `cargo audit` to scan for CVEs
- Run `cargo deny check` for license and deprecation checks
