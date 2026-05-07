# Project Context: Rust Central Auth Service

## Project Overview
A high-performance, **Headless Central Authentication Microservice**. This acts as the single source of truth for identity across multiple GUIs and backend services. The project is a "DIY" implementation focused on memory safety, cryptographic best practices, and modern Rust patterns.

## Tech Stack
- **Language:** Rust (Latest Stable)
- **Web Framework:** `Axum v0.8+` (Public REST API)
- **Internal RPC:** `Tonic` (gRPC) for low-latency, type-safe internal token verification.
- **Database:** `PostgreSQL 17+` via `SQLx` (Compile-time verified queries).
- **Caching/State:** `Redis` for session revocation and OAuth2 nonces.
- **Cryptography:**
    - **Hashing:** `Argon2id` (Modern industry standard for password storage).
    - **Tokens:** `JWT` using `Ed25519` (EdDSA) signatures (Faster/smaller than RSA).
    - **Strategy:** Short-lived Access Tokens + Long-lived Refresh Tokens (stored in Redis).
- **Infrastructure:** Designed for `Shuttle.rs` or Docker-based deployment.

## 🏗 Architecture
1. **Public Face (Axum):** External endpoints for `/register`, `/login`, `/refresh`, and `/logout`.
2. **Private Face (Tonic/gRPC):** Internal `VerifyToken` service called by other microservices.
3. **Data Integrity:** Strict Newtype patterns for IDs and Credentials to prevent type-mixing bugs.

## Directory Structure
```text
.
├── proto/              # gRPC Protobuf definitions (.proto)
├── migrations/         # SQLx migration files
├── tests/              # Integration tests (full HTTP + DB round-trips)
├── src/
│   ├── main.rs         # Server orchestration (Axum + Tonic)
│   ├── routes/         # REST Handlers (Public API)
│   ├── services/       # gRPC Implementations (Internal API)
│   ├── domain/         # Business logic (User hashing, JWT logic)
│   ├── data/           # Repository layer (SQLx queries)
│   └── middleware/     # Auth guards & Rate limiting
├── .env                # Secrets (DB_URL, JWT_PRIVATE_KEY)
└── Cargo.toml
```

## Testing
- Unit tests live in the same file as the code under test, in a `#[cfg(test)]` module at the bottom.
- Integration tests live in `tests/` — spin up the full Axum router and fire real HTTP requests.
- Database tests use `#[sqlx::test]` which provisions an isolated throwaway database per test automatically — never share state between tests.
- No mocking the database. Tests against data layer code must hit a real Postgres instance.
- Run tests: `task test` (requires `task infra:up` first for DB-backed tests).
- **After every code change, write unit tests for the modified logic and integration tests for any affected HTTP endpoints or gRPC services.**

## Security Constraints & Rules
- Input Validation: Every endpoint that accepts a request body must validate all fields before touching the DB, cache, or crypto. Invalid input returns 422. Validation lives in a `validate()` method on the request struct, called at the top of the handler.
- No Manual SQL: All queries must use sqlx::query! or sqlx::query_as! to prevent SQL injection.
- Secure Hashing: Use Argon2id with 2026-standard parameters (e.g., m_cost=64MB).
- Cryptographic Signatures: Use Ed25519 for JWTs. Do not use HS256 (symmetric) in a distributed microservice environment.
- Fail Fast: Use thiserror for internal errors and map them to clean axum::response::IntoResponse types. Never leak database errors to the client.
- Concurrency: Ensure all database and cache interactions are fully asynchronous using tokio.

## Roadmap
- Infrastructure: docker-compose.yml for Postgres/Redis.
- Database: SQLx migrations for the users table.
- Core Crypto: Implement Argon2 hashing and JWT EdDSA signing modules.
- Public API: Implement Axum routes for User Registration and Login.
- Internal API: Implement Tonic gRPC server for token validation.
- Hardening: Add rate limiting and audit logging for failed attempts.

## AI Assistance Instructions
When generating code or providing advice:
- Prioritize Axum 0.8+ syntax and patterns.
- Use idiomatic Rust: Pattern matching over if-let, Result handling over .unwrap().
- Compile-time safety: Always prefer sqlx macros over raw string queries.
- Type-driven development: Suggest using structs and enums to represent domain states.
- No high-level Auth libs: Keep logic DIY (using specific crates like argon2 or jsonwebtoken) rather than suggesting Keycloak or Auth0.
