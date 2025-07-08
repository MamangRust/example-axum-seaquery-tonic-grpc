use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

#[derive(Debug, Deserialize, Serialize, Clone, ToSchema, Validate)]
pub struct CreateCommentRequest {
    #[validate(range(min = 1, message = "Post ID must be greater than 0"))]
    pub id_post_comment: i32,

    #[validate(length(min = 1, message = "User name must not be empty"))]
    pub user_name_comment: String,

    #[validate(length(min = 1, message = "Comment must not be empty"))]
    pub comment: String,
}

#[derive(Debug, Deserialize, Serialize, Clone, ToSchema, Validate)]
pub struct UpdateCommentRequest {
    #[validate(range(min = 1, message = "Post ID must be greater than 0"))]
    pub id_post_comment: i32,

    #[validate(length(min = 1, message = "User name must not be empty"))]
    pub user_name_comment: String,

    #[validate(length(min = 1, message = "Comment must not be empty"))]
    pub comment: String,
}
