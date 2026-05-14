---
paths:
  - "**/*.rs"
---

# Rust Patterns

## Repository Pattern
Encapsulate data access behind a trait; concrete implementations handle storage details:

```rust
pub trait UserRepository: Send + Sync {
    async fn find_by_email(&self, email: &Email) -> Result<Option<User>, DbError>;
    async fn insert(&self, user: &NewUser) -> Result<User, DbError>;
    async fn delete(&self, id: UserId) -> Result<(), DbError>;
}
```

## Service Layer
Business logic in service structs; inject dependencies via constructor:

```rust
pub struct AuthService {
    users: Arc<dyn UserRepository>,
    tokens: Arc<dyn TokenRepository>,
}

impl AuthService {
    pub fn new(users: Arc<dyn UserRepository>, tokens: Arc<dyn TokenRepository>) -> Self {
        Self { users, tokens }
    }
}
```

## Newtype Pattern
Prevent argument mix-ups with distinct wrapper types — this project uses these extensively:

```rust
struct UserId(Uuid);
struct Email(String);
struct PasswordHash(String);

// Can't accidentally pass a raw string where an Email is expected
fn find_user(email: &Email) -> anyhow::Result<Option<User>> { ... }
```

## Enum State Machines
Model states as enums; make illegal states unrepresentable:

```rust
enum TokenState {
    Valid { claims: Claims },
    Expired { expired_at: DateTime<Utc> },
    Revoked,
}
```

Always match exhaustively — no wildcard `_` for business-critical enums.

## Builder Pattern
Use for structs with many optional parameters (config, request builders):

```rust
ServerConfig::builder("0.0.0.0", 8080)
    .max_connections(500)
    .build()
```

## Error Mapping
Define typed errors with `thiserror`; map to HTTP responses via `IntoResponse`:

```rust
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("not found")]
    NotFound,
    #[error("unauthorized")]
    Unauthorized,
    #[error("internal error")]
    Internal,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = match self {
            AppError::NotFound => StatusCode::NOT_FOUND,
            AppError::Unauthorized => StatusCode::UNAUTHORIZED,
            AppError::Internal => StatusCode::INTERNAL_SERVER_ERROR,
        };
        status.into_response()
    }
}
```
