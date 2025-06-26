use std::sync::Arc;

use async_trait::async_trait;
use shared::domain::{ApiResponse, ErrorResponse, LoginRequest, RegisterRequest, UserResponse};

pub type DynAuthService = Arc<dyn AuthServiceTrait + Send + Sync>;

#[async_trait]
pub trait AuthServiceTrait {
    async fn register(
        &self,
        request_data: RegisterRequest,
    ) -> Result<ApiResponse<UserResponse>, ErrorResponse>;
    async fn login(&self, request_data: LoginRequest)
    -> Result<ApiResponse<String>, ErrorResponse>;
    async fn get_me(&self, id: i32) -> Result<ApiResponse<UserResponse>, ErrorResponse>;
}
