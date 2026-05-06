use chrono::{Duration, Utc};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::error::DomainError;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    /// Subject — user UUID.
    pub sub: Uuid,
    pub email: String,
    /// Issued-at (Unix seconds).
    pub iat: i64,
    /// Expiry (Unix seconds).
    pub exp: i64,
    /// Unique token ID — used for refresh-token revocation in Redis.
    pub jti: Uuid,
}

pub struct JwtManager {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    access_ttl: Duration,
    refresh_ttl: Duration,
}

impl JwtManager {
    pub fn from_ed25519_pem(private_pem: &[u8], public_pem: &[u8]) -> Result<Self, DomainError> {
        Ok(Self {
            encoding_key: EncodingKey::from_ed_pem(private_pem)?,
            decoding_key: DecodingKey::from_ed_pem(public_pem)?,
            access_ttl: Duration::minutes(15),
            refresh_ttl: Duration::days(7),
        })
    }

    pub fn sign_access_token(&self, user_id: Uuid, email: &str) -> Result<String, DomainError> {
        self.sign(user_id, email, self.access_ttl)
    }

    pub fn sign_refresh_token(&self, user_id: Uuid, email: &str) -> Result<String, DomainError> {
        self.sign(user_id, email, self.refresh_ttl)
    }

    pub fn verify(&self, token: &str) -> Result<Claims, DomainError> {
        let mut validation = Validation::new(Algorithm::EdDSA);
        validation.validate_exp = true;
        let data = decode::<Claims>(token, &self.decoding_key, &validation)?;
        Ok(data.claims)
    }

    fn sign(&self, user_id: Uuid, email: &str, ttl: Duration) -> Result<String, DomainError> {
        let now = Utc::now();
        let claims = Claims {
            sub: user_id,
            email: email.to_owned(),
            iat: now.timestamp(),
            exp: (now + ttl).timestamp(),
            jti: Uuid::new_v4(),
        };
        Ok(encode(
            &Header::new(Algorithm::EdDSA),
            &claims,
            &self.encoding_key,
        )?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_PRIVATE_PEM: &[u8] = b"-----BEGIN PRIVATE KEY-----
MC4CAQAwBQYDK2VwBCIEIIsgepUW6fIVvsGe3iwBb2mnhBFdIZ7zb+CfdLEo1pNB
-----END PRIVATE KEY-----";

    const TEST_PUBLIC_PEM: &[u8] = b"-----BEGIN PUBLIC KEY-----
MCowBQYDK2VwAyEADyia6fy2lW6Ezrs11/ZGt0axfBAfMSJu+rfdNbu62/Y=
-----END PUBLIC KEY-----";

    fn manager() -> JwtManager {
        JwtManager::from_ed25519_pem(TEST_PRIVATE_PEM, TEST_PUBLIC_PEM).unwrap()
    }

    #[test]
    fn access_token_roundtrip() {
        let mgr = manager();
        let user_id = Uuid::new_v4();
        let token = mgr.sign_access_token(user_id, "alice@example.com").unwrap();
        let claims = mgr.verify(&token).unwrap();
        assert_eq!(claims.sub, user_id);
        assert_eq!(claims.email, "alice@example.com");
    }

    #[test]
    fn refresh_token_roundtrip() {
        let mgr = manager();
        let user_id = Uuid::new_v4();
        let token = mgr.sign_refresh_token(user_id, "bob@example.com").unwrap();
        let claims = mgr.verify(&token).unwrap();
        assert_eq!(claims.sub, user_id);
    }

    #[test]
    fn tampered_token_is_rejected() {
        let mgr = manager();
        let user_id = Uuid::new_v4();
        let mut token = mgr.sign_access_token(user_id, "eve@example.com").unwrap();
        // Flip the last character to corrupt the signature
        let last = token.pop().unwrap();
        token.push(if last == 'A' { 'B' } else { 'A' });
        assert!(mgr.verify(&token).is_err());
    }

    #[test]
    fn each_token_has_unique_jti() {
        let mgr = manager();
        let user_id = Uuid::new_v4();
        let t1 = mgr.sign_access_token(user_id, "x@example.com").unwrap();
        let t2 = mgr.sign_access_token(user_id, "x@example.com").unwrap();
        let c1 = mgr.verify(&t1).unwrap();
        let c2 = mgr.verify(&t2).unwrap();
        assert_ne!(c1.jti, c2.jti);
    }
}
