use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use validator::Validate;

#[derive(Serialize, Deserialize, Clone, Debug, IntoParams)]
pub struct FindAllCategoryRequest {
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

#[derive(Serialize, Deserialize, Clone, Debug, ToSchema, Validate)]
pub struct CreateCategoryRequest {
    #[validate(length(min = 1, message = "Name must not be empty"))]
    pub name: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, ToSchema, Validate)]
pub struct UpdateCategoryRequest {
    #[validate(range(min = 1, message = "ID must be greater than 0"))]
    pub id: i32,

    #[validate(length(min = 1, message = "Name must not be empty"))]
    pub name: String,
}
