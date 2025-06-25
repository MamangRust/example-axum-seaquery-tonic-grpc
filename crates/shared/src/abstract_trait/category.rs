use std::sync::Arc;

use async_trait::async_trait;

use crate::{
    domain::{
        ApiResponse, ApiResponsePagination, CategoryResponse, CreateCategoryRequest, ErrorResponse, FindAllCategoryRequest,  UpdateCategoryRequest
    }, model::category::Category, utils::AppError
    
};

pub type DynCategoryRepository = Arc<dyn CategoryRepositoryTrait + Send + Sync>;
pub type DynCategoryService = Arc<dyn CategoryServiceTrait + Send + Sync>;

#[async_trait]
pub trait CategoryRepositoryTrait {
    async fn find_all(
        &self,
        page: i32,
        page_size: i32,
        search: Option<String>,
    ) -> Result<(Vec<Category>, i64), AppError>;
    async fn find_by_id(&self, id: i32) -> Result<Option<Category>, AppError>;
    async fn create(&self, input: &CreateCategoryRequest) -> Result<Category, AppError>;
    async fn update(&self, input: &UpdateCategoryRequest) -> Result<Category, AppError>;
    async fn delete(&self, id: i32) -> Result<(), AppError>;
}

#[async_trait]
pub trait CategoryServiceTrait {
    async fn get_categories(
        &self,
        req: FindAllCategoryRequest,
    ) -> Result<ApiResponsePagination<Vec<CategoryResponse>>, ErrorResponse>;
    async fn get_category(
        &self,
        id: i32,
    ) -> Result<Option<ApiResponse<CategoryResponse>>, ErrorResponse>;
    async fn create_category(
        &self,
        input: &CreateCategoryRequest,
    ) -> Result<ApiResponse<CategoryResponse>, ErrorResponse>;
    async fn update_category(
        &self,
        input: &UpdateCategoryRequest,
    ) -> Result<Option<ApiResponse<CategoryResponse>>, ErrorResponse>;
    async fn delete_category(&self, id: i32) -> Result<ApiResponse<()>, ErrorResponse>;
}
