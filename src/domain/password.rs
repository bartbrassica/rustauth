use argon2::{
    Argon2, Params,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng},
};

use crate::domain::error::DomainError;

pub struct PasswordService {
    argon2: Argon2<'static>,
}

impl PasswordService {
    pub fn new() -> Self {
        // 64 MB memory, 3 iterations, 4 lanes — 2026 OWASP recommendation for Argon2id
        let params = Params::new(64 * 1024, 3, 4, None).expect("valid argon2 params");
        Self {
            argon2: Argon2::new(argon2::Algorithm::Argon2id, argon2::Version::V0x13, params),
        }
    }

    pub fn hash(&self, password: &str) -> Result<String, DomainError> {
        let salt = SaltString::generate(&mut OsRng);
        let hash = self.argon2.hash_password(password.as_bytes(), &salt)?;
        Ok(hash.to_string())
    }

    pub fn verify(&self, password: &str, hash: &str) -> Result<bool, DomainError> {
        let parsed = PasswordHash::new(hash)?;
        Ok(self
            .argon2
            .verify_password(password.as_bytes(), &parsed)
            .is_ok())
    }
}

impl Default for PasswordService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_and_verify_roundtrip() {
        let svc = PasswordService::new();
        let hash = svc.hash("hunter2").unwrap();
        assert!(svc.verify("hunter2", &hash).unwrap());
        assert!(!svc.verify("wrong_password", &hash).unwrap());
    }

    #[test]
    fn different_passwords_produce_different_hashes() {
        let svc = PasswordService::new();
        let h1 = svc.hash("password").unwrap();
        let h2 = svc.hash("password").unwrap();
        // Argon2 uses a random salt each time
        assert_ne!(h1, h2);
    }
}
