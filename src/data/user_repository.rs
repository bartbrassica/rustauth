use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::data::error::DataError;

#[derive(Debug, Clone)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub password_hash: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct UserRepository<'a> {
    pool: &'a PgPool,
}

impl<'a> UserRepository<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, email: &str, password_hash: &str) -> Result<User, DataError> {
        sqlx::query_as!(
            User,
            "INSERT INTO users (email, password_hash) VALUES ($1, $2) RETURNING *",
            email,
            password_hash,
        )
        .fetch_one(self.pool)
        .await
        .map_err(DataError::from_sqlx)
    }

    pub async fn find_by_email(&self, email: &str) -> Result<Option<User>, DataError> {
        sqlx::query_as!(User, "SELECT * FROM users WHERE email = $1", email)
            .fetch_optional(self.pool)
            .await
            .map_err(DataError::from_sqlx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[sqlx::test]
    async fn create_and_find_by_email(pool: PgPool) {
        let repo = UserRepository::new(&pool);
        let user = repo
            .create("alice@example.com", "hashed_password")
            .await
            .unwrap();

        assert_eq!(user.email, "alice@example.com");

        let found = repo
            .find_by_email("alice@example.com")
            .await
            .unwrap()
            .expect("user should exist");

        assert_eq!(found.id, user.id);
        assert_eq!(found.password_hash, "hashed_password");
    }

    #[sqlx::test]
    async fn find_by_email_returns_none_for_unknown(pool: PgPool) {
        let repo = UserRepository::new(&pool);
        let result = repo.find_by_email("ghost@example.com").await.unwrap();
        assert!(result.is_none());
    }

    #[sqlx::test]
    async fn duplicate_email_returns_email_conflict(pool: PgPool) {
        let repo = UserRepository::new(&pool);
        repo.create("bob@example.com", "hash1").await.unwrap();
        let err = repo.create("bob@example.com", "hash2").await.unwrap_err();
        assert!(matches!(err, DataError::EmailConflict));
    }
}
