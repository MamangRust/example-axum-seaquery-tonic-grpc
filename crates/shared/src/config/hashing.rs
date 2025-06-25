use bcrypt::{hash, verify, BcryptError};
use crate::utils::AppError;

#[derive(Clone, Debug)]
pub struct Hashing;

impl Hashing {
    pub fn new() -> Self {
        Hashing
    }

    pub async fn hash_password(&self, password: &str) -> Result<String, BcryptError> {
        hash(password, 4)
    }

    pub async fn compare_password(&self, hashed_password: &str, password: &str) -> Result<(), AppError> {
        match verify(password, hashed_password) {
            Ok(true) => Ok(()),
            Ok(false) => Err(AppError::HashingError(BcryptError::from(std::io::Error::new(std::io::ErrorKind::Other, "Passwords do not match.")))), 
            Err(e) => Err(AppError::BcryptError(e.to_string())),
        }
    }
}
