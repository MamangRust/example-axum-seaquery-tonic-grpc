use async_trait::async_trait;
use std::sync::Arc;

use shared::domain::{
    ApiResponse, CommentResponse, CreateCommentRequest, ErrorResponse, UpdateCommentRequest,
};

pub type DynCommentService = Arc<dyn CommentServiceTrait + Send + Sync>;

#[async_trait]
pub trait CommentServiceTrait {
    async fn find_all(&self) -> Result<ApiResponse<Vec<CommentResponse>>, ErrorResponse>;
    async fn find_by_id(&self, id: &i32) -> Result<ApiResponse<CommentResponse>, ErrorResponse>;
    async fn create(
        &self,
        req: &CreateCommentRequest,
    ) -> Result<ApiResponse<CommentResponse>, ErrorResponse>;
    async fn update(
        &self,
        req: &UpdateCommentRequest,
    ) -> Result<ApiResponse<CommentResponse>, ErrorResponse>;
    async fn delete(&self, id: &i32) -> Result<ApiResponse<()>, ErrorResponse>;
}
