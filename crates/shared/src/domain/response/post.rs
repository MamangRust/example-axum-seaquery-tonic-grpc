use serde::Serialize;
use utoipa::ToSchema;

use crate::model::posts::{PostRelationModel, Post};
use genproto::post::{PostResponse as ProtoPostResponse, PostRelationResponse as ProtoPostRelationResponse};


#[derive(Debug, Serialize, ToSchema)]
pub struct PostResponse {
    pub id: i32,
    pub title: String,
    pub body: String,
    pub img: String,
    pub category_id: i32,
    pub user_id: i32,
    pub user_name: String,
}

impl From<Post> for PostResponse {
    fn from(post: Post) -> Self {
        PostResponse {
            id: post.id,
            title: post.title,
            body: post.body,
            img: post.img,
            category_id: post.category_id,
            user_id: post.user_id,
            user_name: post.user_name,
        }
    }
}

impl From<PostResponse> for ProtoPostResponse {
    fn from(post: PostResponse) -> Self {
        ProtoPostResponse {
            id: post.id,
            title: post.title,
            body: post.body,
            img: post.img,
            category_id: post.category_id,
            user_id: post.user_id,
            user_name: post.user_name,
        }
    }
}

impl From<ProtoPostResponse> for PostResponse {
    fn from(post: ProtoPostResponse) -> Self {
        PostResponse {
            id: post.id,
            title: post.title,
            body: post.body,
            img: post.img,
            category_id: post.category_id,
            user_id: post.user_id,
            user_name: post.user_name,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PostRelationResponse {
    pub post_id: i32,
    pub title: String,
    pub comment_id: i32,
    pub id_post_comment: i32,
    pub user_name_comment: String,
    pub comment: String,
}



impl From<PostRelationModel> for PostRelationResponse {
    fn from(post_relation: PostRelationModel) -> Self {
        PostRelationResponse {
            post_id: post_relation.post_id,
            title: post_relation.title,
            comment_id: post_relation.comment_id,
            id_post_comment: post_relation.id_post_comment,
            user_name_comment: post_relation.user_name_comment,
            comment: post_relation.comment,
        }
    }
}

impl From<PostRelationResponse> for ProtoPostRelationResponse {
    fn from(post_relation: PostRelationResponse) -> Self {
        ProtoPostRelationResponse {
            post_id: post_relation.post_id,
            title: post_relation.title,
            comment_id: post_relation.comment_id,
            id_post_comment: post_relation.id_post_comment,
            user_name_comment: post_relation.user_name_comment,
            comment: post_relation.comment,
        }
    }
}

impl From<ProtoPostRelationResponse> for PostRelationResponse {
    fn from(post_relation: ProtoPostRelationResponse) -> Self {
        PostRelationResponse {
            post_id: post_relation.post_id,
            title: post_relation.title,
            comment_id: post_relation.comment_id,
            id_post_comment: post_relation.id_post_comment,
            user_name_comment: post_relation.user_name_comment,
            comment: post_relation.comment,
        }
    }
}