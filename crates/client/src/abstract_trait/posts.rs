use async_trait::async_trait;
use std::sync::Arc;

use shared::domain::{
    ApiResponse, ApiResponsePagination, CreatePostRequest, ErrorResponse, FindAllPostRequest,
    PostRelationResponse, PostResponse, UpdatePostRequest,
};

pub type DynPostsService = Arc<dyn PostsServiceTrait + Send + Sync>;

#[async_trait]
pub trait PostsServiceTrait {
    async fn find_all(
        &self,
        req: &FindAllPostRequest,
    ) -> Result<ApiResponsePagination<Vec<PostResponse>>, ErrorResponse>;
    async fn find_by_id(&self, id: &i32) -> Result<ApiResponse<PostResponse>, ErrorResponse>;
    async fn create(
        &self,
        req: &CreatePostRequest,
    ) -> Result<ApiResponse<PostResponse>, ErrorResponse>;
    async fn update(
        &self,
        req: &UpdatePostRequest,
    ) -> Result<ApiResponse<PostResponse>, ErrorResponse>;
    async fn find_relation(
        &self,
        id: &i32,
    ) -> Result<ApiResponse<PostRelationResponse>, ErrorResponse>;
    async fn delete(&self, id: &i32) -> Result<ApiResponse<()>, ErrorResponse>;
}
