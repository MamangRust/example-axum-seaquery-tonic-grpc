use async_trait::async_trait;
use std::sync::Arc;

use shared::domain::{
    ApiResponse, ApiResponsePagination, CategoryResponse, CreateCategoryRequest, ErrorResponse,
    FindAllCategoryRequest, UpdateCategoryRequest,
};

pub type DynCategoryService = Arc<dyn CategoryServiceTrait + Send + Sync>;

#[async_trait]
pub trait CategoryServiceTrait {
    async fn find_all(
        &self,
        req: &FindAllCategoryRequest,
    ) -> Result<ApiResponsePagination<Vec<CategoryResponse>>, ErrorResponse>;
    async fn find_by_id(&self, id: &i32) -> Result<ApiResponse<CategoryResponse>, ErrorResponse>;
    async fn create(
        &self,
        req: &CreateCategoryRequest,
    ) -> Result<ApiResponse<CategoryResponse>, ErrorResponse>;
    async fn update(
        &self,
        req: &UpdateCategoryRequest,
    ) -> Result<ApiResponse<CategoryResponse>, ErrorResponse>;
    async fn delete(&self, id: &i32) -> Result<ApiResponse<()>, ErrorResponse>;
}
