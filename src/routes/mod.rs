use std::net::SocketAddr;

use axum::{
    Json,
    extract::{ConnectInfo, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    AppState,
    data::{DataError, TokenStore, UserRepository},
    domain::DomainError,
};

// --- /register ---

#[derive(Deserialize)]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct RegisterResponse {
    pub id: Uuid,
    pub email: String,
}

pub async fn register(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<AppState>,
    Json(body): Json<RegisterRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let hash = state.passwords.hash(&body.password)?;
    let repo = UserRepository::new(&state.pool);
    let user = repo.create(&body.email, &hash).await.map_err(|e| {
        if matches!(e, DataError::EmailConflict) {
            tracing::warn!(email = %body.email, ip = %addr.ip(), event = "register_failed", reason = "email_conflict");
        }
        ApiError::from(e)
    })?;
    tracing::info!(email = %user.email, user_id = %user.id, ip = %addr.ip(), event = "register_success");
    Ok((
        StatusCode::CREATED,
        Json(RegisterResponse {
            id: user.id,
            email: user.email,
        }),
    ))
}

// --- /login ---

#[derive(Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct LoginResponse {
    pub access_token: String,
    pub refresh_token: String,
}

pub async fn login(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<AppState>,
    Json(body): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, ApiError> {
    let repo = UserRepository::new(&state.pool);
    let user = match repo.find_by_email(&body.email).await? {
        Some(u) => u,
        None => {
            tracing::warn!(email = %body.email, ip = %addr.ip(), event = "login_failed", reason = "unknown_email");
            return Err(ApiError::Unauthorized);
        }
    };

    if !state
        .passwords
        .verify(&body.password, &user.password_hash)?
    {
        tracing::warn!(email = %body.email, ip = %addr.ip(), event = "login_failed", reason = "invalid_password");
        return Err(ApiError::Unauthorized);
    }

    let access_token = state.jwt.sign_access_token(user.id, &user.email)?;
    let (refresh_token, refresh_jti) = state.jwt.sign_refresh_token(user.id, &user.email)?;

    TokenStore::new(&state.redis)
        .store_refresh_token(refresh_jti, user.id, 7 * 24 * 3600)
        .await?;

    tracing::info!(email = %user.email, user_id = %user.id, ip = %addr.ip(), event = "login_success");
    Ok(Json(LoginResponse {
        access_token,
        refresh_token,
    }))
}

// --- /refresh ---

#[derive(Deserialize)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

pub async fn refresh(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<AppState>,
    Json(body): Json<RefreshRequest>,
) -> Result<Json<LoginResponse>, ApiError> {
    let claims = state.jwt.verify_refresh(&body.refresh_token).map_err(|_| {
        tracing::warn!(ip = %addr.ip(), event = "refresh_failed", reason = "invalid_token");
        ApiError::Unauthorized
    })?;

    let store = TokenStore::new(&state.redis);
    // Atomically consume the old JTI — None means already revoked or unknown.
    store
        .revoke_refresh_token(claims.jti)
        .await?
        .ok_or_else(|| {
            tracing::warn!(user_id = %claims.sub, ip = %addr.ip(), event = "refresh_failed", reason = "token_revoked");
            ApiError::Unauthorized
        })?;

    let access_token = state.jwt.sign_access_token(claims.sub, &claims.email)?;
    let (refresh_token, new_jti) = state.jwt.sign_refresh_token(claims.sub, &claims.email)?;

    store
        .store_refresh_token(new_jti, claims.sub, 7 * 24 * 3600)
        .await?;

    tracing::info!(user_id = %claims.sub, ip = %addr.ip(), event = "token_refreshed");
    Ok(Json(LoginResponse {
        access_token,
        refresh_token,
    }))
}

// --- /logout ---

#[derive(Deserialize)]
pub struct LogoutRequest {
    pub refresh_token: String,
}

pub async fn logout(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<AppState>,
    Json(body): Json<LogoutRequest>,
) -> Result<StatusCode, ApiError> {
    let claims = state.jwt.verify_refresh(&body.refresh_token).map_err(|_| {
        tracing::warn!(ip = %addr.ip(), event = "logout_failed", reason = "invalid_token");
        ApiError::Unauthorized
    })?;
    TokenStore::new(&state.redis)
        .revoke_refresh_token(claims.jti)
        .await?;
    tracing::info!(user_id = %claims.sub, ip = %addr.ip(), event = "logout");
    Ok(StatusCode::NO_CONTENT)
}

// --- Error type ---

pub enum ApiError {
    Conflict,
    Unauthorized,
    Internal,
}

impl From<DomainError> for ApiError {
    fn from(e: DomainError) -> Self {
        match e {
            DomainError::InvalidToken(_) | DomainError::WrongTokenKind => Self::Unauthorized,
            DomainError::Hashing(_) => Self::Internal,
        }
    }
}

impl From<DataError> for ApiError {
    fn from(e: DataError) -> Self {
        match e {
            DataError::EmailConflict => Self::Conflict,
            DataError::NotFound | DataError::Database(_) | DataError::Cache(_) => Self::Internal,
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            Self::Conflict => (StatusCode::CONFLICT, "email already registered"),
            Self::Unauthorized => (StatusCode::UNAUTHORIZED, "invalid credentials"),
            Self::Internal => (StatusCode::INTERNAL_SERVER_ERROR, "internal error"),
        };
        (status, message).into_response()
    }
}
