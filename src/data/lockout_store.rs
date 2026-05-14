use redis::{AsyncCommands, Client};

use super::error::DataError;

const LOCKOUT_MAX: u32 = 10;
const LOCKOUT_WINDOW_SECS: i64 = 900; // 15 minutes, sliding from last failure

pub struct LockoutStore<'a> {
    client: &'a Client,
}

impl<'a> LockoutStore<'a> {
    pub fn new(client: &'a Client) -> Self {
        Self { client }
    }

    pub async fn is_locked(&self, email: &str) -> Result<bool, DataError> {
        let mut conn = self.client.get_multiplexed_async_connection().await?;
        let count: Option<u32> = conn.get(format!("lockout:{email}")).await?;
        Ok(count.unwrap_or(0) >= LOCKOUT_MAX)
    }

    /// Increments the failure counter and refreshes the lockout window. Returns the new count.
    pub async fn record_failure(&self, email: &str) -> Result<u32, DataError> {
        let mut conn = self.client.get_multiplexed_async_connection().await?;
        let key = format!("lockout:{email}");
        let count: u32 = conn.incr(&key, 1u32).await?;
        let _: () = conn.expire(&key, LOCKOUT_WINDOW_SECS).await?;
        Ok(count)
    }

    pub async fn clear(&self, email: &str) -> Result<(), DataError> {
        let mut conn = self.client.get_multiplexed_async_connection().await?;
        let _: () = conn.del(format!("lockout:{email}")).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_client() -> Client {
        let url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".into());
        Client::open(url).expect("test redis url")
    }

    #[tokio::test]
    async fn not_locked_with_no_failures() {
        let client = test_client();
        let store = LockoutStore::new(&client);
        let email = format!("test-{}@example.com", uuid::Uuid::new_v4());
        assert!(!store.is_locked(&email).await.unwrap());
    }

    #[tokio::test]
    async fn locked_after_max_failures() {
        let client = test_client();
        let store = LockoutStore::new(&client);
        let email = format!("test-{}@example.com", uuid::Uuid::new_v4());

        for _ in 0..LOCKOUT_MAX {
            store.record_failure(&email).await.unwrap();
        }
        assert!(store.is_locked(&email).await.unwrap());

        let mut conn = client.get_multiplexed_async_connection().await.unwrap();
        let _: () = conn.del(format!("lockout:{email}")).await.unwrap();
    }

    #[tokio::test]
    async fn clear_resets_lock() {
        let client = test_client();
        let store = LockoutStore::new(&client);
        let email = format!("test-{}@example.com", uuid::Uuid::new_v4());

        for _ in 0..LOCKOUT_MAX {
            store.record_failure(&email).await.unwrap();
        }
        assert!(store.is_locked(&email).await.unwrap());

        store.clear(&email).await.unwrap();
        assert!(!store.is_locked(&email).await.unwrap());
    }

    #[tokio::test]
    async fn record_failure_returns_incrementing_count() {
        let client = test_client();
        let store = LockoutStore::new(&client);
        let email = format!("test-{}@example.com", uuid::Uuid::new_v4());

        assert_eq!(store.record_failure(&email).await.unwrap(), 1);
        assert_eq!(store.record_failure(&email).await.unwrap(), 2);
        assert_eq!(store.record_failure(&email).await.unwrap(), 3);

        let mut conn = client.get_multiplexed_async_connection().await.unwrap();
        let _: () = conn.del(format!("lockout:{email}")).await.unwrap();
    }
}
