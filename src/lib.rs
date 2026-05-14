pub mod data;
pub mod domain;
pub mod middleware;
pub mod routes;
pub mod services;

use std::sync::Arc;

use axum::{
    Router, middleware as mw,
    routing::{get, patch, post},
};

use domain::{JwtManager, PasswordService};

#[derive(Clone)]
pub struct AppState {
    pub pool: sqlx::PgPool,
    pub jwt: Arc<JwtManager>,
    pub passwords: Arc<PasswordService>,
    pub redis: redis::Client,
}

/// Router without rate limiting — for integration tests.
pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(routes::health))
        .route("/register", post(routes::register))
        .route("/login", post(routes::login))
        .route("/refresh", post(routes::refresh))
        .route("/logout", post(routes::logout))
        .route("/me", get(routes::me).delete(routes::delete_me))
        .route("/me/password", patch(routes::change_password))
        .with_state(state)
}

/// Production router with per-IP rate limiting on /register and /login.
pub fn build_production_router(state: AppState) -> Router {
    let rate_limited = Router::new()
        .route("/login", post(routes::login))
        .route("/register", post(routes::register))
        .route_layer(mw::from_fn_with_state(
            state.clone(),
            middleware::rate_limit,
        ));

    Router::new()
        .route("/health", get(routes::health))
        .merge(rate_limited)
        .route("/refresh", post(routes::refresh))
        .route("/logout", post(routes::logout))
        .route("/me", get(routes::me).delete(routes::delete_me))
        .route("/me/password", patch(routes::change_password))
        .with_state(state)
}
