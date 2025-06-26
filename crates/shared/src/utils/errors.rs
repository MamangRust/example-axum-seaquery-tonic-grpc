use bcrypt::BcryptError;
use jsonwebtoken::errors::Error as JwtError;
use serde::Serialize;
use sqlx::Error as SqlxError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Database error: {0}")]
    SqlxError(#[from] SqlxError),

    #[error("Hashing error: {0}")]
    HashingError(#[from] BcryptError),

    #[error("Invalid credentials")]
    InvalidCredentials,

    #[error("Token expired")]
    TokenExpiredError,

    #[error("Token validation error")]
    TokenValidationError,

    #[error("Token generation error: {0}")]
    TokenGenerationError(#[from] JwtError),

    #[error("Bcrypt error: {0}")]
    BcryptError(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Email already exists")]
    EmailAlreadyExists,

    #[error("Validation error: {0}")]
    ValidationError(String),
}

impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        serializer.serialize_str(self.to_string().as_ref())
    }
}
