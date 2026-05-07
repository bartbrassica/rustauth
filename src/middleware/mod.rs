use axum::{
    extract::FromRequestParts,
    http::{StatusCode, header, request::Parts},
    response::{IntoResponse, Response},
};

use crate::{AppState, domain::Claims};

/// Extractor that validates the `Authorization: Bearer <token>` header
/// and injects the verified [`Claims`] into the handler.
pub struct AuthUser(pub Claims);

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = AuthError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let auth_header = parts
            .headers
            .get(header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .ok_or(AuthError::MissingToken)?;

        let token = auth_header
            .strip_prefix("Bearer ")
            .ok_or(AuthError::MissingToken)?;

        let claims = state
            .jwt
            .verify(token)
            .map_err(|_| AuthError::InvalidToken)?;
        Ok(AuthUser(claims))
    }
}

pub enum AuthError {
    MissingToken,
    InvalidToken,
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let (status, body) = match self {
            Self::MissingToken => (StatusCode::UNAUTHORIZED, "missing authorization token"),
            Self::InvalidToken => (StatusCode::UNAUTHORIZED, "invalid or expired token"),
        };
        (status, body).into_response()
    }
}
