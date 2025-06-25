use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

#[derive(Serialize, Deserialize, Clone, Debug, IntoParams)]
pub struct FindAllPostRequest {
    #[serde(default = "default_page")]
    pub page: i32,

    #[serde(default = "default_page_size")]
    pub page_size: i32,

    #[serde(default)]
    pub search: String,
}

fn default_page() -> i32 {
    1
}

fn default_page_size() -> i32 {
    10
}

#[derive(Debug, Deserialize, Serialize, ToSchema, Clone)]
pub struct CreatePostRequest {
    pub title: String,
    pub body: String,
    #[schema(format = Binary, content_media_type = "application/octet-stream")]
    pub file: String,
    pub category_id: i32,
    pub user_id: i32,
    pub user_name: String,
}

#[derive(Debug, Deserialize, Serialize, ToSchema, Clone)]
pub struct UpdatePostRequest {
    pub post_id: Option<i32>,
    pub title: String,
    pub body: String,
    #[schema(format = Binary, content_media_type = "application/octet-stream")]
    pub file: String,
    pub category_id: i32,
    pub user_id: i32,
    pub user_name: String,
}
