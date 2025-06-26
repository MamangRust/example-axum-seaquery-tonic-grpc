use serde::Serialize;
use utoipa::ToSchema;

use crate::model::category::Category;
use genproto::category::CategoryResponse as ProtoCategoryResponse;

#[derive(Debug, Serialize, ToSchema)]
#[allow(non_snake_case)]
pub struct CategoryResponse {
    pub id: i32,
    pub name: String,
}

impl From<Category> for CategoryResponse {
    fn from(category: Category) -> Self {
        CategoryResponse {
            id: category.id,
            name: category.name,
        }
    }
}

impl From<CategoryResponse> for ProtoCategoryResponse {
    fn from(category: CategoryResponse) -> Self {
        ProtoCategoryResponse {
            id: category.id,
            name: category.name,
        }
    }
}

impl From<ProtoCategoryResponse> for CategoryResponse {
    fn from(category: ProtoCategoryResponse) -> Self {
        CategoryResponse {
            id: category.id,
            name: category.name,
        }
    }
}

impl From<Option<ProtoCategoryResponse>> for CategoryResponse {
    fn from(category: Option<ProtoCategoryResponse>) -> Self {
        match category {
            Some(category) => category.into(),
            None => CategoryResponse {
                id: 0,
                name: "".to_string(),
            },
        }
    }
}
