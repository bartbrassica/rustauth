use redis::{AsyncCommands, Client};
use uuid::Uuid;

use super::error::DataError;

pub struct TokenStore<'a> {
    client: &'a Client,
}

impl<'a> TokenStore<'a> {
    pub fn new(client: &'a Client) -> Self {
        Self { client }
    }

    pub async fn store_refresh_token(
        &self,
        jti: Uuid,
        user_id: Uuid,
        ttl_secs: u64,
    ) -> Result<(), DataError> {
        let mut conn = self.client.get_multiplexed_async_connection().await?;
        let key = format!("refresh:{jti}");
        let _: () = conn.set_ex(key, user_id.to_string(), ttl_secs).await?;
        Ok(())
    }

    pub async fn revoke_refresh_token(&self, jti: Uuid) -> Result<Option<Uuid>, DataError> {
        let mut conn = self.client.get_multiplexed_async_connection().await?;
        let key = format!("refresh:{jti}");
        let val: Option<String> = conn.get_del(key).await?;
        Ok(val.and_then(|s| s.parse().ok()))
    }
}
