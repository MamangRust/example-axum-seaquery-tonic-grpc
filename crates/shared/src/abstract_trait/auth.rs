use std::sync::Arc;

use async_trait::async_trait;

use crate::{domain::{ApiResponse, ErrorResponse, LoginRequest, RegisterRequest, UserResponse}, utils::AppError};


pub type DynAuthService = Arc<dyn AuthServiceTrait + Send + Sync>;

#[async_trait]
pub trait AuthServiceTrait {
    async fn register_user(&self, input: &RegisterRequest) -> Result<ApiResponse<UserResponse>, ErrorResponse>;
    async fn login_user(&self, input: &LoginRequest) -> Result<ApiResponse<String>, ErrorResponse>;
    fn verify_token(&self, token: &str) -> Result<i64, AppError>;
}