use crate::{middleware::jwt, state::AppState};
use axum::{
    extract::{Json, Multipart, Path, Query, State},
    http::StatusCode,
    middleware,
    response::IntoResponse,
    routing::{delete, get, post, put},
};
use serde_json::json;
use shared::domain::{
    ApiResponse, ApiResponsePagination, CreatePostRequest, FindAllPostRequest,
    PostRelationResponse, PostResponse, UpdatePostRequest,
};
use std::sync::Arc;
use utoipa_axum::router::OpenApiRouter;

#[utoipa::path(
    get,
    path = "/api/posts",
    params(FindAllPostRequest),
    responses(
        (status = 200, description = "List all posts successfully", body = ApiResponsePagination<Vec<PostResponse>>)
    ),
    security(("bearer_auth" = [])),
    tag = "posts"
)]
pub async fn get_posts(
    State(data): State<Arc<AppState>>,
    Query(params): Query<FindAllPostRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    match data.di_container.post_service.find_all(&params).await {
        Ok(posts) => Ok((StatusCode::OK, Json(json!(posts)))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(json!(e)))),
    }
}

#[utoipa::path(
    get,
    path = "/api/posts/{id}",
    params(
        ("id" = i32, Path, description = "Post ID")
    ),
    responses(
        (status = 200, description = "Get post by ID", body = ApiResponse<PostResponse>),
        (status = 404, description = "Post not found")
    ),
    tag = "posts"
)]
pub async fn get_post(
    State(data): State<Arc<AppState>>,
    Path(post_id): Path<i32>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    match data.di_container.post_service.find_by_id(&post_id).await {
        Ok(post) => Ok((StatusCode::OK, Json(json!(post)))),
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
                "message": "Failed to fetch post",
                "error": e.message
            })),
        )),
    }
}

#[utoipa::path(
    get,
    path = "/api/posts/{id}/relation",
    params(
            ("id" = i32, Path, description = "Post ID")
    ),
    responses(
        (status = 200, description = "Get related posts", body = ApiResponse<Vec<PostRelationResponse>>),
        (status = 404, description = "Post not found")
    ),
    tag = "posts"
)]
pub async fn get_post_relation(
    State(data): State<Arc<AppState>>,
    Path(post_id): Path<i32>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    match data.di_container.post_service.find_relation(&post_id).await {
        Ok(posts) => Ok((StatusCode::OK, Json(json!(posts)))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(json!(e)))),
    }
}

#[utoipa::path(
    post,
    path = "/api/posts/create",
    request_body(content = CreatePostRequest, content_type = "multipart/form-data"),
    responses(
        (status = 201, description = "Post created successfully", body = ApiResponse<PostResponse>),
        (status = 400, description = "Invalid request body"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "posts"
)]
pub async fn create_post(
    State(data): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let mut title: Option<String> = None;
    let mut body: Option<String> = None;
    let mut category_id: Option<i32> = None;
    let mut user_id: Option<i32> = None;
    let mut user_name: Option<String> = None;
    let mut file_data: Option<(String, String, Vec<u8>)> = None;

    while let Some(field) = multipart.next_field().await.unwrap() {
        match field.name() {
            Some("title") => {
                title = Some(field.text().await.expect("should be text for title field"));
            }
            Some("body") => {
                body = Some(field.text().await.expect("should be text for body field"));
            }
            Some("category_id") => {
                category_id = Some(
                    field
                        .text()
                        .await
                        .expect("should be text for category_id field")
                        .parse()
                        .expect("should be a number for category_id field"),
                );
            }
            Some("user_id") => {
                user_id = Some(
                    field
                        .text()
                        .await
                        .expect("should be text for user_id field")
                        .parse()
                        .expect("should be a number for user_id field"),
                );
            }
            Some("user_name") => {
                user_name = Some(
                    field
                        .text()
                        .await
                        .expect("should be text for user_name field"),
                );
            }
            Some("file") => {
                let file_name = field.file_name().map(ToString::to_string);
                let content_type = field.content_type().map(ToString::to_string);
                let bytes = field.bytes().await.expect("should be bytes for file field");

                if let (Some(name), Some(content_type)) = (file_name, content_type) {
                    if !bytes.is_empty() {
                        file_data = Some((name, content_type, bytes.to_vec()));
                    }
                }
            }
            _ => (),
        };
    }

    let uploaded_file_name = if let Some((name, content_type, file_bytes)) = file_data {
        let upload_result = data
            .di_container
            .file_service
            .upload_image("posts", name, content_type, file_bytes)
            .await;

        match upload_result {
            Ok(response) => response.file_name.clone(),
            Err((status, response)) => {
                return Err((
                    status,
                    Json(json!({
                        "error": response.message
                    })),
                ));
            }
        }
    } else {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": "File is required"
            })),
        ));
    };

    let post_data = CreatePostRequest {
        title: title.unwrap_or_default(),
        body: body.unwrap_or_default(),
        file: uploaded_file_name,
        category_id: category_id.unwrap_or(0),
        user_id: user_id.unwrap_or(0),
        user_name: user_name.unwrap_or_default(),
    };

    match data.di_container.post_service.create(&post_data).await {
        Ok(post) => Ok((StatusCode::CREATED, Json(json!(post)))),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        )),
    }
}

#[utoipa::path(
    put,
    path = "/api/posts/update/{id}",
    params(
        ("id" = i32, Path, description = "Post ID")
    ),
    request_body(content = UpdatePostRequest, content_type = "multipart/form-data"),
    responses(
        (status = 200, description = "Post updated successfully", body = ApiResponse<PostResponse>),
        (status = 400, description = "Invalid request body"),
        (status = 404, description = "Post not found")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "posts"
)]
pub async fn update_post(
    State(data): State<Arc<AppState>>,
    Path(post_id): Path<i32>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let mut title: Option<String> = None;
    let mut body: Option<String> = None;
    let mut category_id: Option<i32> = None;
    let mut user_id: Option<i32> = None;
    let mut user_name: Option<String> = None;
    let mut file_data: Option<(String, String, Vec<u8>)> = None;

    let old_post = match data.di_container.post_service.find_by_id(&post_id).await {
        Ok(post) => post.data,
        Err(e) => return Err((StatusCode::NOT_FOUND, Json(json!(e)))),
    };

    if !old_post.img.is_empty() {
        let _ = data
            .di_container
            .file_service
            .delete_image("posts", &old_post.img)
            .await;
    }

    while let Some(field) = multipart.next_field().await.unwrap() {
        match field.name() {
            Some("title") => {
                title = Some(field.text().await.expect("should be text for title field"));
            }
            Some("body") => {
                body = Some(field.text().await.expect("should be text for body field"));
            }
            Some("category_id") => {
                category_id = Some(
                    field
                        .text()
                        .await
                        .expect("should be text for category_id field")
                        .parse()
                        .expect("should be a number for category_id field"),
                );
            }
            Some("user_id") => {
                user_id = Some(
                    field
                        .text()
                        .await
                        .expect("should be text for user_id field")
                        .parse()
                        .expect("should be a number for user_id field"),
                );
            }
            Some("user_name") => {
                user_name = Some(
                    field
                        .text()
                        .await
                        .expect("should be text for user_name field"),
                );
            }
            Some("file") => {
                let file_name = field.file_name().map(ToString::to_string);
                let content_type = field.content_type().map(ToString::to_string);
                let bytes = field.bytes().await.expect("should be bytes for file field");

                if let (Some(name), Some(content_type)) = (file_name, content_type) {
                    if !bytes.is_empty() {
                        file_data = Some((name, content_type, bytes.to_vec()));
                    }
                }
            }
            _ => (),
        };
    }

    let uploaded_file_name = if let Some((name, content_type, file_bytes)) = file_data {
        let upload_result = data
            .di_container
            .file_service
            .upload_image("posts", name, content_type, file_bytes)
            .await;

        match upload_result {
            Ok(response) => response.file_name.clone(),
            Err((status, response)) => {
                return Err((
                    status,
                    Json(json!({
                        "error": response.message
                    })),
                ));
            }
        }
    } else {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": "File is required"
            })),
        ));
    };

    let post_data = UpdatePostRequest {
        post_id: Some(post_id),
        title: title.unwrap_or_default(),
        body: body.unwrap_or_default(),
        file: uploaded_file_name,
        category_id: category_id.unwrap_or(0),
        user_id: user_id.unwrap_or(0),
        user_name: user_name.unwrap_or_default(),
    };

    match data.di_container.post_service.update(&post_data).await {
        Ok(post) => Ok((StatusCode::OK, Json(json!(post)))),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        )),
    }
}

#[utoipa::path(
    delete,
    path = "/api/posts/delete/{id}",
    params(
        ("id" = i32, Path, description = "Category ID")
    ),
    responses(
        (status = 200, description = "Post deleted successfully"),
        (status = 404, description = "Post not found"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "posts"
)]
pub async fn delete_post(
    State(data): State<Arc<AppState>>,
    Path(post_id): Path<i32>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    match data.di_container.post_service.delete(&post_id).await {
        Ok(_) => Ok((
            StatusCode::OK,
            Json(json!({
                "status": "success",
                "message": "Post deleted successfully"
            })),
        )),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(json!(e)))),
    }
}

pub fn post_routes(app_state: Arc<AppState>) -> OpenApiRouter {
    let protected_routes = OpenApiRouter::new()
        .route("/api/posts/create", post(create_post))
        .route("/api/posts/{id}", get(get_post))
        .route("/api/posts/update/{id}", put(update_post))
        .route("/api/posts/delete/{id}", delete(delete_post))
        .route("/api/posts/{id}/relation", get(get_post_relation))
        .route_layer(middleware::from_fn_with_state(app_state.clone(), jwt::auth))
        .with_state(app_state.clone());

    let public_routes = OpenApiRouter::new().route("/posts", get(get_posts));

    OpenApiRouter::new()
        .merge(protected_routes)
        .merge(public_routes)
        .with_state(app_state)
}
