use crate::{
    middleware::{jwt, validate::SimpleValidatedJson},
    state::AppState,
};
use axum::{
    Extension,
    extract::{Json, Path, Query, State},
    http::StatusCode,
    middleware,
    response::IntoResponse,
    routing::{delete, get, post, put},
};
use serde_json::json;
use shared::domain::{
    ApiResponse, ApiResponsePagination, CategoryResponse, CreateCategoryRequest,
    FindAllCategoryRequest, UpdateCategoryRequest,
};
use std::sync::Arc;
use utoipa_axum::router::OpenApiRouter;

#[utoipa::path(
    get,
    path = "/api/categories",
    params(FindAllCategoryRequest),
    responses(
        (status = 200, description = "List all category successfully", body = ApiResponsePagination<Vec<CategoryResponse>>)
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "category"
)]
pub async fn get_categories(
    State(data): State<Arc<AppState>>,
    Query(params): Query<FindAllCategoryRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    match data.di_container.category_service.find_all(&params).await {
        Ok(categories) => Ok((StatusCode::OK, Json(json!(categories)))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(json!(e)))),
    }
}

#[utoipa::path(
    get,
    path = "/api/categories/{id}",
    tag = "Categories",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("id" = i32, Path, description = "Category ID")
    ),
    responses(
        (status = 200, description = "Successfully retrieved category details", body = ApiResponse<CategoryResponse>),
        (status = 404, description = "Category not found", body = serde_json::Value),
        (status = 500, description = "Internal server error", body = serde_json::Value),
    )
)]
pub async fn get_category(
    State(data): State<Arc<AppState>>,
    Path(id): Path<i32>,
    Extension(_user_id): Extension<i64>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    match data.di_container.category_service.find_by_id(&id).await {
        Ok(category) => Ok((StatusCode::OK, Json(json!(category)))),
        Err(e) if e.status == "NOT_FOUND" => Err((
            StatusCode::NOT_FOUND,
            Json(json!({
                "status": "fail",
                "message": e.message
            })),
        )),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "status": "error",
                "message": "Failed to fetch category",
                "error": e.message
            })),
        )),
    }
}

#[utoipa::path(
    post,
    path = "/api/categories/create",
    responses(
        (status = 200, description = "Create category", body = ApiResponse<CategoryResponse>)
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "category"
)]
pub async fn create_category(
    State(data): State<Arc<AppState>>,
    SimpleValidatedJson(body): SimpleValidatedJson<CreateCategoryRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    match data.di_container.category_service.create(&body).await {
        Ok(category) => Ok((StatusCode::CREATED, Json(json!(category)))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(json!(e)))),
    }
}

#[utoipa::path(
    put,
    path = "/api/categories/update/{id}",
    params(
        ("id" = i32, Path, description = "Category ID")
    ),
    responses(
        (status = 200, description = "Update category", body = ApiResponse<CategoryResponse>),
        (status = 404, description = "Category not found"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "category"
)]
pub async fn update_category(
    State(data): State<Arc<AppState>>,
    Path(id): Path<i32>,
    SimpleValidatedJson(mut body): SimpleValidatedJson<UpdateCategoryRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    body.id = id;

    match data.di_container.category_service.update(&body).await {
        Ok(category) => Ok((StatusCode::OK, Json(json!(category)))),
        Err(e) if e.status == "NOT_FOUND" => Err((
            StatusCode::NOT_FOUND,
            Json(json!({
                "status": "fail",
                "message": e.message
            })),
        )),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "status": "error",
                "message": "Failed to update category",
                "error": e.message
            })),
        )),
    }
}

#[utoipa::path(
    delete,
    path = "/api/categories/delete/{id}",
    params(
        ("id" = i32, Path, description = "Category ID")
    ),
    responses(
        (status = 200, description = "Delete category", body = Value)
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "category"
)]
pub async fn delete_category(
    State(data): State<Arc<AppState>>,
    Path(id): Path<i32>,
    Extension(_user_id): Extension<i64>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    match data.di_container.category_service.delete(&id).await {
        Ok(_) => Ok((
            StatusCode::OK,
            Json(json!({
                "status": "success",
                "message": "Category deleted successfully"
            })),
        )),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(json!(e)))),
    }
}

pub fn category_routes(app_state: Arc<AppState>) -> OpenApiRouter {
    let protected_routes = OpenApiRouter::new()
        .route("/api/categories/{id}", get(get_category))
        .route("/api/categories/create", post(create_category))
        .route("/api/categories/update/{id}", put(update_category))
        .route("/api/categories/delete/{id}", delete(delete_category))
        .route_layer(middleware::from_fn_with_state(app_state.clone(), jwt::auth))
        .with_state(app_state.clone());

    let public_routes = OpenApiRouter::new().route("/api/categories", get(get_categories));

    OpenApiRouter::new()
        .merge(protected_routes)
        .merge(public_routes)
        .with_state(app_state.clone())
}
