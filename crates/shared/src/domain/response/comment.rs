use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::model::comment::Comment;
use genproto::comment::CommentResponse as ProtoCommentResponse;

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct CommentResponse {
    pub id: i32,
    pub id_post_comment: i32,
    pub user_name_comment: String,
    pub comment: String,
}

impl From<Comment> for CommentResponse {
    fn from(comment: Comment) -> Self {
        CommentResponse {
            id: comment.id,
            id_post_comment: comment.id_post_comment,
            user_name_comment: comment.user_name_comment,
            comment: comment.comment,
        }
    }
}

impl From<CommentResponse> for ProtoCommentResponse {
    fn from(comment: CommentResponse) -> Self {
        ProtoCommentResponse {
            id: comment.id,
            id_post_comment: comment.id_post_comment,
            user_name_comment: comment.user_name_comment,
            comment: comment.comment,
        }
    }
}

impl From<ProtoCommentResponse> for CommentResponse {
    fn from(comment: ProtoCommentResponse) -> Self {
        CommentResponse {
            id: comment.id,
            id_post_comment: comment.id_post_comment,
            user_name_comment: comment.user_name_comment,
            comment: comment.comment,
        }
    }
}