use std::sync::Arc;
use async_trait::async_trait;

use crate::{domain::{ApiResponse, ApiResponsePagination, FindAllUserRequest,CreateUserRequest, ErrorResponse, UpdateUserRequest, UserResponse},  model::user::User, utils::AppError};


pub type DynUserRepository = Arc<dyn UserRepositoryTrait + Send + Sync>;
pub type DynUserService = Arc<dyn UserServiceTrait + Send + Sync>;

#[async_trait]
pub trait UserRepositoryTrait {
    async fn find_all(
        &self,
        page: i32,
        page_size: i32,
        search: Option<String>,
    ) -> Result<(Vec<User>, i64), AppError>;
    async fn find_by_email_exists(&self, email: &str) -> Result<bool, AppError>;
    async fn create_user(
        &self,
        input: &CreateUserRequest
    ) -> Result<User, AppError>;
    async fn find_by_email(&self, email: &str) -> Result<Option<User>, AppError>;
    async fn find_by_id(&self, id: i32) -> Result<Option<User>, AppError>;
    async fn update_user(
        &self,
        input: &UpdateUserRequest
    ) -> Result<User, AppError>;
    async fn delete_user(&self, email: &str) -> Result<(), AppError>;
}

#[async_trait]
pub trait UserServiceTrait {
    async fn get_users(
        &self, 
        req: FindAllUserRequest,
    ) -> Result<ApiResponsePagination<Vec<UserResponse>>, ErrorResponse>;
    async fn create_user(
        &self,
        input: &CreateUserRequest
    ) -> Result<ApiResponse<UserResponse>, ErrorResponse>;
    async fn find_by_id(&self, id: i32) -> Result<Option<ApiResponse<UserResponse>>, ErrorResponse>;
    async fn update_user(
        &self,
        input: &UpdateUserRequest
    ) -> Result<Option<ApiResponse<UserResponse>>, ErrorResponse>;
    async fn delete_user(&self, email: &str) -> Result<ApiResponse<()>, ErrorResponse>;
}