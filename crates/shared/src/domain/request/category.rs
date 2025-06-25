use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

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

#[derive(Serialize, Deserialize, Clone, Debug, ToSchema)]
pub struct CreateCategoryRequest {
    pub name: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, ToSchema)]
pub struct UpdateCategoryRequest {
    pub id: Option<i32>,
    pub name: Option<String>,
}
