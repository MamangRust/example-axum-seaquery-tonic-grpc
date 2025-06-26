use std::sync::Arc;

use async_trait::async_trait;

use crate::{
    domain::{
        ApiResponse, CommentResponse, CreateCommentRequest, ErrorResponse, UpdateCommentRequest,
    },
    model::comment::Comment,
    utils::AppError,
};

pub type DynCommentRepository = Arc<dyn CommentRepositoryTrait + Send + Sync>;
pub type DynCommentService = Arc<dyn CommentServiceTrait + Send + Sync>;

#[async_trait]
pub trait CommentRepositoryTrait {
    async fn find_all(&self) -> Result<Vec<Comment>, AppError>;
    async fn find_by_id(&self, id: i32) -> Result<Option<Comment>, AppError>;
    async fn create(&self, input: &CreateCommentRequest) -> Result<Comment, AppError>;
    async fn update(&self, input: &UpdateCommentRequest) -> Result<Comment, AppError>;
    async fn delete(&self, id: i32) -> Result<(), AppError>;
}

#[async_trait]
pub trait CommentServiceTrait {
    async fn get_comments(&self) -> Result<ApiResponse<Vec<CommentResponse>>, ErrorResponse>;
    async fn get_comment(
        &self,
        id: i32,
    ) -> Result<Option<ApiResponse<CommentResponse>>, ErrorResponse>;
    async fn create_comment(
        &self,
        input: &CreateCommentRequest,
    ) -> Result<ApiResponse<CommentResponse>, ErrorResponse>;
    async fn update_comment(
        &self,
        input: &UpdateCommentRequest,
    ) -> Result<Option<ApiResponse<CommentResponse>>, ErrorResponse>;
    async fn delete_comment(&self, id: i32) -> Result<ApiResponse<()>, ErrorResponse>;
}
