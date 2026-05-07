#[derive(Debug, thiserror::Error)]
pub enum DataError {
    #[error("not found")]
    NotFound,
    #[error("email already registered")]
    EmailConflict,
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("cache error: {0}")]
    Cache(redis::RedisError),
}

impl From<redis::RedisError> for DataError {
    fn from(e: redis::RedisError) -> Self {
        Self::Cache(e)
    }
}

impl DataError {
    pub(super) fn from_sqlx(e: sqlx::Error) -> Self {
        // Postgres unique-violation code is "23505"
        if let sqlx::Error::Database(ref db) = e
            && db.code().as_deref() == Some("23505")
        {
            return Self::EmailConflict;
        }
        Self::Database(e)
    }
}
