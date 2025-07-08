use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use validator::Validate;

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

#[derive(Debug, Deserialize, Serialize, ToSchema, Validate, Clone)]
pub struct CreatePostRequest {
    #[validate(length(min = 3, message = "Title must be at least 3 characters"))]
    pub title: String,

    #[validate(length(min = 10, message = "Body must be at least 10 characters"))]
    pub body: String,

    #[schema(format = Binary, content_media_type = "application/octet-stream")]
    #[validate(length(min = 1, message = "File must not be empty"))]
    pub file: String,

    pub category_id: i32,
    pub user_id: i32,

    #[validate(length(min = 1, message = "User name is required"))]
    pub user_name: String,
}

#[derive(Debug, Deserialize, Serialize, ToSchema, Validate, Clone)]
pub struct UpdatePostRequest {
    pub post_id: i32,

    #[validate(length(min = 3, message = "Title must be at least 3 characters"))]
    pub title: String,

    #[validate(length(min = 10, message = "Body must be at least 10 characters"))]
    pub body: String,

    #[schema(format = Binary, content_media_type = "application/octet-stream")]
    #[validate(length(min = 1, message = "File must not be empty"))]
    pub file: String,

    pub category_id: i32,
    pub user_id: i32,

    #[validate(length(min = 1, message = "User name is required"))]
    pub user_name: String,
}
