use serde::{Deserialize, Serialize};
use sqlx::prelude::FromRow;

#[derive(Debug, FromRow, Serialize, Deserialize, Clone)]
pub struct Post {
    pub id: i32,
    pub title: String,
    pub img: String,
    pub body: String,
    pub category_id: i32,
    pub user_id: i32,
    pub user_name: String,
}

#[derive(Debug, sqlx::FromRow, Serialize, Deserialize, Clone)]
pub struct PostRelationModel {
    pub post_id: i32,
    pub title: String,
    pub comment_id: i32,
    pub id_post_comment: i32,
    pub user_name_comment: String,
    pub comment: String,
}
