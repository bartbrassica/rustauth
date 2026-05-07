mod data;
mod domain;
mod middleware;
mod routes;
mod services;

use std::sync::Arc;

use axum::{Router, routing::post};
use dotenvy::dotenv;
use sqlx::postgres::PgPoolOptions;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use domain::{JwtManager, PasswordService};

#[derive(Clone)]
pub struct AppState {
    pub pool: sqlx::PgPool,
    pub jwt: Arc<JwtManager>,
    pub passwords: Arc<PasswordService>,
    pub redis: redis::Client,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "rustauth=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let database_url = std::env::var("DATABASE_URL")?;
    let redis_url = std::env::var("REDIS_URL")?;
    let jwt_private_pem = std::env::var("JWT_PRIVATE_KEY_PEM")?;
    let jwt_public_pem = std::env::var("JWT_PUBLIC_KEY_PEM")?;
    let http_addr = std::env::var("HTTP_ADDR").unwrap_or_else(|_| "0.0.0.0:3000".to_string());

    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&database_url)
        .await?;

    sqlx::migrate!().run(&pool).await?;
    tracing::info!("migrations applied");

    let redis = redis::Client::open(redis_url)?;

    let jwt = Arc::new(JwtManager::from_ed25519_pem(
        jwt_private_pem.as_bytes(),
        jwt_public_pem.as_bytes(),
    )?);

    let passwords = Arc::new(PasswordService::new());

    let state = AppState {
        pool,
        jwt,
        passwords,
        redis,
    };

    let app = Router::new()
        .route("/register", post(routes::register))
        .route("/login", post(routes::login))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(&http_addr).await?;
    tracing::info!("listening on {http_addr}");
    axum::serve(listener, app).await?;

    Ok(())
}
