use crate::utils::AppError;
use bcrypt::{BcryptError, hash, verify};

#[derive(Clone, Debug)]
pub struct Hashing;

impl Hashing {
    pub async fn hash_password(&self, password: &str) -> anyhow::Result<String, BcryptError> {
        hash(password, 4)
    }

    pub async fn compare_password(
        &self,
        hashed_password: &str,
        password: &str,
    ) -> anyhow::Result<(), AppError> {
        match verify(password, hashed_password) {
            Ok(true) => Ok(()),
            Ok(false) => Err(AppError::HashingError(BcryptError::from(
                std::io::Error::other("Passwords do not match."),
            ))),
            Err(e) => Err(AppError::BcryptError(e.to_string())),
        }
    }
}
