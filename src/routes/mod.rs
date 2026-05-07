use axum::{
    Json,
    extract::State,
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
    State(state): State<AppState>,
    Json(body): Json<RegisterRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let hash = state.passwords.hash(&body.password)?;
    let repo = UserRepository::new(&state.pool);
    let user = repo.create(&body.email, &hash).await?;
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
    State(state): State<AppState>,
    Json(body): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, ApiError> {
    let repo = UserRepository::new(&state.pool);
    let user = repo
        .find_by_email(&body.email)
        .await?
        .ok_or(ApiError::Unauthorized)?;

    if !state
        .passwords
        .verify(&body.password, &user.password_hash)?
    {
        return Err(ApiError::Unauthorized);
    }

    let access_token = state.jwt.sign_access_token(user.id, &user.email)?;
    let (refresh_token, refresh_jti) = state.jwt.sign_refresh_token(user.id, &user.email)?;

    TokenStore::new(&state.redis)
        .store_refresh_token(refresh_jti, user.id, 7 * 24 * 3600)
        .await?;

    Ok(Json(LoginResponse {
        access_token,
        refresh_token,
    }))
}

// --- Error type ---

pub enum ApiError {
    Conflict,
    Unauthorized,
    Internal,
}

impl From<DomainError> for ApiError {
    fn from(_: DomainError) -> Self {
        Self::Internal
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
