mod data;
mod domain;
mod middleware;
mod routes;
mod services;

use dotenvy::dotenv;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

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

    // TODO: parse config from env (HTTP_ADDR, GRPC_ADDR, DATABASE_URL, REDIS_URL)
    // TODO: sqlx::PgPool::connect(&database_url).await?
    // TODO: redis::Client::open(redis_url)?
    // TODO: run sqlx migrations
    // TODO: tokio::join! Axum HTTP server + Tonic gRPC server

    tracing::info!("rustauth starting");

    Ok(())
}
