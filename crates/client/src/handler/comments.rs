use crate::{
    middleware::{jwt, validate::SimpleValidatedJson},
    state::AppState,
};
use axum::{
    extract::{Json, Path, State},
    http::StatusCode,
    middleware,
    response::IntoResponse,
    routing::{delete, get, post, put},
};
use serde_json::json;
use shared::domain::{ApiResponse, CommentResponse, CreateCommentRequest, UpdateCommentRequest};
use std::sync::Arc;
use utoipa_axum::router::OpenApiRouter;

#[utoipa::path(
    get,
    path = "/api/comments",
    responses(
        (status = 200, description = "Get all comments", body = ApiResponse<Vec<CommentResponse>>)
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "comments"
)]
pub async fn get_comments(
    State(data): State<Arc<AppState>>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    match data.di_container.comment_service.find_all().await {
        Ok(comments) => Ok((StatusCode::OK, Json(json!(comments)))),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "status": "error",
                "message": "Failed to fetch comments",
                "error": format!("{:?}", e)
            })),
        )),
    }
}

#[utoipa::path(
    get,
    path = "/api/comments/{id}",
    responses(
        (status = 200, description = "Get a comment", body = ApiResponse<CommentResponse>),
        (status = 404, description = "Comment not found")
    ),
    params(
        ("id" = i32, Path, description = "Comment ID")
    ),
    tag = "comments"
)]
pub async fn get_comment(
    State(data): State<Arc<AppState>>,
    Path(comment_id): Path<i32>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    match data
        .di_container
        .comment_service
        .find_by_id(&comment_id)
        .await
    {
        Ok(comment) => Ok((StatusCode::OK, Json(json!(comment)))),
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
                "message": "Failed to fetch comment",
                "error": e.message
            })),
        )),
    }
}

#[utoipa::path(
    post,
    path = "/api/comments",
    request_body = CreateCommentRequest,
    responses(
        (status = 201, description = "Comment created", body = ApiResponse<CommentResponse>),
        (status = 400, description = "Invalid request body")
    ),
    tag = "comments"
)]
pub async fn create_comment(
    State(data): State<Arc<AppState>>,
    Json(body): Json<CreateCommentRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    match data.di_container.comment_service.create(&body).await {
        Ok(comment) => Ok((StatusCode::CREATED, Json(json!(comment)))),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "status": "error",
                "message": "Failed to create comment",
                "error": format!("{:?}", e)
            })),
        )),
    }
}

#[utoipa::path(
    put,
    path = "/api/comments/{id}",
    request_body = UpdateCommentRequest,
    responses(
        (status = 200, description = "Comment updated", body = ApiResponse<CommentResponse>),
        (status = 404, description = "Comment not found")
    ),
    params(
        ("id" = i32, Path, description = "Comment ID")
    ),
    tag = "comments"
)]
pub async fn update_comment(
    State(data): State<Arc<AppState>>,
    Path(id): Path<i32>,
    SimpleValidatedJson(mut body): SimpleValidatedJson<UpdateCommentRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    body.id_post_comment = id;

    match data.di_container.comment_service.update(&body).await {
        Ok(comment) => Ok((StatusCode::OK, Json(json!(comment)))),
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
                "message": "Failed to update comment",
                "error": e.message
            })),
        )),
    }
}

#[utoipa::path(
    delete,
    path = "/api/comments/{id}",
    responses(
        (status = 200, description = "Comment deleted successfully", body=Value),
        (status = 500, description = "Failed to delete comment")
    ),
    params(
        ("id" = i32, Path, description = "Comment ID")
    ),
    tag = "comments"
)]
pub async fn delete_comment(
    State(data): State<Arc<AppState>>,
    Path(comment_id): Path<i32>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    match data.di_container.comment_service.delete(&comment_id).await {
        Ok(_) => Ok((
            StatusCode::OK,
            Json(json!({
                "status": "success",
                "message": "Comment deleted successfully"
            })),
        )),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "status": "error",
                "message": "Failed to delete comment",
                "error": format!("{:?}", e)
            })),
        )),
    }
}

pub fn comment_routes(app_state: Arc<AppState>) -> OpenApiRouter {
    let protected_routes = OpenApiRouter::new()
        .route("/api/comments", get(get_comments))
        .route("/api/comments/{id}", get(get_comment))
        .route("/api/comments", post(create_comment))
        .route("/api/comments/{id}", put(update_comment))
        .route("/api/comments/{id}", delete(delete_comment))
        .route_layer(middleware::from_fn_with_state(app_state.clone(), jwt::auth))
        .with_state(app_state.clone());

    OpenApiRouter::new()
        .merge(protected_routes)
        .with_state(app_state)
}
