use std::{net::SocketAddr, sync::Arc};

use sqlx::PgPool;
use tokio::net::TcpListener;

use rustauth::{
    AppState, build_router,
    domain::{JwtManager, PasswordService},
};

// Same test keys as in the jwt unit tests.
const TEST_PRIVATE_PEM: &[u8] = b"-----BEGIN PRIVATE KEY-----
MC4CAQAwBQYDK2VwBCIEIIsgepUW6fIVvsGe3iwBb2mnhBFdIZ7zb+CfdLEo1pNB
-----END PRIVATE KEY-----";

const TEST_PUBLIC_PEM: &[u8] = b"-----BEGIN PUBLIC KEY-----
MCowBQYDK2VwAyEADyia6fy2lW6Ezrs11/ZGt0axfBAfMSJu+rfdNbu62/Y=
-----END PUBLIC KEY-----";

/// Builds the app with a test DB, connects to Redis, and binds to a random
/// port. Returns the base URL. The server runs for the lifetime of the test.
async fn spawn_app(pool: PgPool) -> String {
    let redis_url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".into());
    let redis = redis::Client::open(redis_url).expect("valid redis url");

    let jwt = Arc::new(
        JwtManager::from_ed25519_pem(TEST_PRIVATE_PEM, TEST_PUBLIC_PEM)
            .expect("valid test keypair"),
    );
    let passwords = Arc::new(PasswordService::for_testing());

    let state = AppState {
        pool,
        jwt,
        passwords,
        redis,
    };
    let app = build_router(state);

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .await
        .unwrap();
    });

    format!("http://{addr}")
}

// --- /register ---

#[sqlx::test]
async fn register_returns_201_with_user_info(pool: PgPool) {
    let base = spawn_app(pool).await;
    let client = reqwest::Client::new();

    let res = client
        .post(format!("{base}/register"))
        .json(&serde_json::json!({"email": "alice@example.com", "password": "hunter2"}))
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), 201);
    let body: serde_json::Value = res.json().await.unwrap();
    assert_eq!(body["email"], "alice@example.com");
    assert!(body["id"].is_string());
}

#[sqlx::test]
async fn register_duplicate_email_returns_409(pool: PgPool) {
    let base = spawn_app(pool).await;
    let client = reqwest::Client::new();
    let payload = serde_json::json!({"email": "alice@example.com", "password": "hunter2"});

    client
        .post(format!("{base}/register"))
        .json(&payload)
        .send()
        .await
        .unwrap();
    let res = client
        .post(format!("{base}/register"))
        .json(&payload)
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), 409);
}

// --- /login ---

#[sqlx::test]
async fn login_with_valid_credentials_returns_tokens(pool: PgPool) {
    let base = spawn_app(pool).await;
    let client = reqwest::Client::new();

    client
        .post(format!("{base}/register"))
        .json(&serde_json::json!({"email": "alice@example.com", "password": "hunter2"}))
        .send()
        .await
        .unwrap();

    let res = client
        .post(format!("{base}/login"))
        .json(&serde_json::json!({"email": "alice@example.com", "password": "hunter2"}))
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), 200);
    let body: serde_json::Value = res.json().await.unwrap();
    assert!(body["access_token"].is_string());
    assert!(body["refresh_token"].is_string());
}

#[sqlx::test]
async fn login_with_wrong_password_returns_401(pool: PgPool) {
    let base = spawn_app(pool).await;
    let client = reqwest::Client::new();

    client
        .post(format!("{base}/register"))
        .json(&serde_json::json!({"email": "alice@example.com", "password": "hunter2"}))
        .send()
        .await
        .unwrap();

    let res = client
        .post(format!("{base}/login"))
        .json(&serde_json::json!({"email": "alice@example.com", "password": "wrongpass"}))
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), 401);
}

#[sqlx::test]
async fn login_with_unknown_email_returns_401(pool: PgPool) {
    let base = spawn_app(pool).await;

    let res = reqwest::Client::new()
        .post(format!("{base}/login"))
        .json(&serde_json::json!({"email": "ghost@example.com", "password": "anything"}))
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), 401);
}

// --- /me ---

#[sqlx::test]
async fn me_with_valid_access_token_returns_user(pool: PgPool) {
    let base = spawn_app(pool).await;
    let client = reqwest::Client::new();

    client
        .post(format!("{base}/register"))
        .json(&serde_json::json!({"email": "alice@example.com", "password": "hunter2"}))
        .send()
        .await
        .unwrap();

    let login: serde_json::Value = client
        .post(format!("{base}/login"))
        .json(&serde_json::json!({"email": "alice@example.com", "password": "hunter2"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    let res = client
        .get(format!("{base}/me"))
        .bearer_auth(login["access_token"].as_str().unwrap())
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), 200);
    let body: serde_json::Value = res.json().await.unwrap();
    assert_eq!(body["email"], "alice@example.com");
    assert!(body["id"].is_string());
}

#[sqlx::test]
async fn me_without_token_returns_401(pool: PgPool) {
    let base = spawn_app(pool).await;

    let res = reqwest::Client::new()
        .get(format!("{base}/me"))
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), 401);
}

#[sqlx::test]
async fn me_rejects_refresh_token(pool: PgPool) {
    let base = spawn_app(pool).await;
    let client = reqwest::Client::new();

    client
        .post(format!("{base}/register"))
        .json(&serde_json::json!({"email": "alice@example.com", "password": "hunter2"}))
        .send()
        .await
        .unwrap();

    let login: serde_json::Value = client
        .post(format!("{base}/login"))
        .json(&serde_json::json!({"email": "alice@example.com", "password": "hunter2"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    let res = client
        .get(format!("{base}/me"))
        .bearer_auth(login["refresh_token"].as_str().unwrap())
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), 401);
}

// --- /refresh ---

#[sqlx::test]
async fn refresh_issues_new_tokens_and_revokes_old(pool: PgPool) {
    let base = spawn_app(pool).await;
    let client = reqwest::Client::new();

    client
        .post(format!("{base}/register"))
        .json(&serde_json::json!({"email": "alice@example.com", "password": "hunter2"}))
        .send()
        .await
        .unwrap();

    let login: serde_json::Value = client
        .post(format!("{base}/login"))
        .json(&serde_json::json!({"email": "alice@example.com", "password": "hunter2"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    let old_refresh = login["refresh_token"].as_str().unwrap();

    let refreshed: serde_json::Value = client
        .post(format!("{base}/refresh"))
        .json(&serde_json::json!({"refresh_token": old_refresh}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    assert!(refreshed["access_token"].is_string());
    assert_ne!(refreshed["refresh_token"], login["refresh_token"]);

    // Old refresh token must be revoked — replay must fail.
    let replay = client
        .post(format!("{base}/refresh"))
        .json(&serde_json::json!({"refresh_token": old_refresh}))
        .send()
        .await
        .unwrap();

    assert_eq!(replay.status(), 401);
}

#[sqlx::test]
async fn refresh_rejects_access_token(pool: PgPool) {
    let base = spawn_app(pool).await;
    let client = reqwest::Client::new();

    client
        .post(format!("{base}/register"))
        .json(&serde_json::json!({"email": "alice@example.com", "password": "hunter2"}))
        .send()
        .await
        .unwrap();

    let login: serde_json::Value = client
        .post(format!("{base}/login"))
        .json(&serde_json::json!({"email": "alice@example.com", "password": "hunter2"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    let res = client
        .post(format!("{base}/refresh"))
        .json(&serde_json::json!({"refresh_token": login["access_token"]}))
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), 401);
}

// --- /logout ---

#[sqlx::test]
async fn logout_returns_204_and_revokes_refresh_token(pool: PgPool) {
    let base = spawn_app(pool).await;
    let client = reqwest::Client::new();

    client
        .post(format!("{base}/register"))
        .json(&serde_json::json!({"email": "alice@example.com", "password": "hunter2"}))
        .send()
        .await
        .unwrap();

    let login: serde_json::Value = client
        .post(format!("{base}/login"))
        .json(&serde_json::json!({"email": "alice@example.com", "password": "hunter2"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    let refresh_token = login["refresh_token"].as_str().unwrap();

    let logout = client
        .post(format!("{base}/logout"))
        .json(&serde_json::json!({"refresh_token": refresh_token}))
        .send()
        .await
        .unwrap();

    assert_eq!(logout.status(), 204);

    // Revoked token must be rejected on next refresh attempt.
    let replay = client
        .post(format!("{base}/refresh"))
        .json(&serde_json::json!({"refresh_token": refresh_token}))
        .send()
        .await
        .unwrap();

    assert_eq!(replay.status(), 401);
}
