use chrono::{Duration, Utc};
use jsonwebtoken::{
    decode, encode, errors::ErrorKind as JwtError, DecodingKey, EncodingKey, Header, Validation,
};
use serde::{Serialize, Deserialize};

use crate::utils::AppError;



#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub user_id: i64,
    pub exp: usize,
    pub iat: usize,
}

impl Claims {
    pub fn new(user_id: i64, exp: usize, iat: usize) -> Self {
        Claims { user_id, exp, iat}
    }
}

#[derive(Clone, Debug)]
pub struct JwtConfig{
    pub jwt_secret: String,
}

impl JwtConfig {
    pub fn new(jwt_secret: &str) -> Self {
        JwtConfig{
            jwt_secret: jwt_secret.to_string(),
        }
    }

    pub fn generate_token(&self, user_id: i64) -> Result<String, AppError> {
        let now = Utc::now();
        let iat = now.timestamp() as usize;
        let exp = (now + Duration::minutes(60)).timestamp() as usize;

        let claims = Claims::new(user_id, exp, iat);

        match encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret("YOUR_SECRET_KEY".as_ref()),
        ) {
            Ok(token) => Ok(token),
            Err(err) => Err(AppError::TokenGenerationError(err.into())),
        }
    }

    pub fn verify_token(&self, token: &str) -> Result<i64, AppError> {
        let decoding_key = DecodingKey::from_secret("YOUR_SECRET_KEY".as_ref());
    
        match decode::<Claims>(token, &decoding_key, &Validation::default()) {
            Ok(token_data) => {
                let current_time = Utc::now().timestamp() as usize;
    
                if token_data.claims.exp >= current_time {
                    Ok(token_data.claims.user_id)
                } else {
                    Err(AppError::TokenExpiredError)
                }
            }
            Err(err) => {
                if let JwtError::ExpiredSignature = err.kind() {
                    Err(AppError::TokenExpiredError)
                } else {
                    eprintln!("Error decoding token: {:?}", err);
                    Err(AppError::TokenValidationError)
                }
            }
        }
    }
    
}