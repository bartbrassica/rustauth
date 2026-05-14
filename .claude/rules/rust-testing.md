---
paths:
  - "**/*.rs"
---

# Rust Testing

## Framework
- `#[test]` with `#[cfg(test)]` modules for unit tests
- `#[tokio::test]` for async unit tests
- `#[sqlx::test]` for database tests — provisions an isolated throwaway DB per test automatically
- `rstest` for parameterized tests
- `proptest` for property-based testing

## Test Organization
```
src/
  domain/user.rs        # unit tests in #[cfg(test)] mod at the bottom
  data/user_repo.rs     # #[sqlx::test] tests at the bottom
tests/
  register.rs           # integration tests: full HTTP round-trips against real Axum router
  login.rs
```

## Rules
- **No mocking the database.** Data layer tests must hit a real Postgres instance via `#[sqlx::test]`.
- Never share state between tests — each `#[sqlx::test]` gets its own isolated DB.
- Integration tests spin up the full Axum router and fire real HTTP requests.
- After every code change: write unit tests for modified logic + integration tests for affected endpoints.

## Test Naming
Describe the scenario, not the implementation:
- `creates_user_with_valid_email`
- `rejects_duplicate_email`
- `returns_401_for_invalid_password`

## Parameterized Tests
```rust
use rstest::rstest;

#[rstest]
#[case("", "email is required")]
#[case("notanemail", "invalid email format")]
fn rejects_invalid_email(#[case] input: &str, #[case] expected_msg: &str) {
    let result = Email::parse(input);
    assert!(result.unwrap_err().to_string().contains(expected_msg));
}
```

## Running Tests
```bash
task infra:up   # start Postgres + Redis
task test       # run full suite
cargo test --lib                    # unit tests only
cargo test --test register          # specific integration test
```
