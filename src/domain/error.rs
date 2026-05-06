use argon2::password_hash;

#[derive(Debug, thiserror::Error)]
pub enum DomainError {
    #[error("password hashing failed: {0}")]
    Hashing(password_hash::Error),
    #[error("invalid token: {0}")]
    InvalidToken(#[from] jsonwebtoken::errors::Error),
}

impl From<password_hash::Error> for DomainError {
    fn from(e: password_hash::Error) -> Self {
        Self::Hashing(e)
    }
}
