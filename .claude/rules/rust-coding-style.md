---
paths:
  - "**/*.rs"
---

# Rust Coding Style

## Formatting & Lints
- `cargo fmt` before every commit (hook runs automatically)
- `cargo clippy -- -D warnings` — treat all warnings as errors
- Max line width: 100 characters
- 4-space indent

## Immutability
- `let` by default; `let mut` only when mutation is required
- Prefer returning new values over mutating in place
- Use `Cow<'_, T>` when a function may or may not need to allocate

## Naming
- `snake_case` for functions, methods, variables, modules, crates
- `PascalCase` for types, traits, enums, type parameters
- `SCREAMING_SNAKE_CASE` for constants and statics

## Ownership & Borrowing
- Borrow by default; take ownership only when necessary
- Never clone without understanding the root cause
- Accept `&str` over `String`, `&[T]` over `Vec<T>` in function parameters
- Use `impl Into<String>` for ownership-requiring constructors

## Error Handling
- Use `Result<T, E>` and `?` for propagation
- Define typed errors with `thiserror` (this is a library/service, not an app)
- Add context with `.with_context()` where helpful
- Reserve `.unwrap()` for tests and states that are logically unreachable

## Iterators vs Loops
- Prefer iterator chains for transformations
- Use explicit loops only for complex control flow that doesn't read clearly as a chain

## Module Organization
- Organize by domain, not by type
- Default to private; use `pub(crate)` for internal sharing
- Only mark `pub` for items that are part of the public API
