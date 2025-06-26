use async_trait::async_trait;
use shared::domain::{
    ApiResponse, ApiResponsePagination, CreateUserRequest, ErrorResponse, FindAllUserRequest,
    UpdateUserRequest, UserResponse,
};
use std::sync::Arc;

pub type DynUserService = Arc<dyn UserServiceTrait + Send + Sync>;

#[async_trait]
pub trait UserServiceTrait {
    async fn find_all(
        &self,
        req: &FindAllUserRequest,
    ) -> Result<ApiResponsePagination<Vec<UserResponse>>, ErrorResponse>;
    async fn find_by_id(&self, id: &i32) -> Result<ApiResponse<UserResponse>, ErrorResponse>;
    async fn create(
        &self,
        req: &CreateUserRequest,
    ) -> Result<ApiResponse<UserResponse>, ErrorResponse>;
    async fn update(
        &self,
        req: &UpdateUserRequest,
    ) -> Result<ApiResponse<UserResponse>, ErrorResponse>;
    async fn delete(&self, email: &str) -> Result<ApiResponse<()>, ErrorResponse>;
}
