use crate::{
    abstract_trait::{DynPostsRepository, PostsServiceTrait},
    cache::CacheStore,
    domain::{
        ApiResponse, ApiResponsePagination, CreatePostRequest, ErrorResponse, FindAllPostRequest,
        Pagination, PostRelationResponse, PostResponse, UpdatePostRequest,
    },
    utils::{AppError, MetadataInjector, Method, Metrics, Status as StatusUtils, TracingContext},
};
use async_trait::async_trait;
use opentelemetry::{
    Context, KeyValue,
    global::{self, BoxedTracer},
    trace::{Span, SpanKind, TraceContextExt, Tracer},
};
use prometheus_client::registry::Registry;
use std::{sync::Arc, time::Duration};
use tokio::{sync::Mutex, time::Instant};
use tonic::Request;
use tracing::{error, info};

#[derive(Clone)]
pub struct PostService {
    repository: DynPostsRepository,
    metrics: Arc<Mutex<Metrics>>,
    cache_store: Arc<CacheStore>,
}

impl std::fmt::Debug for PostService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PostService")
            .field("repository", &"DynPostsRepository")
            .finish()
    }
}

impl PostService {
    pub async fn new(
        repository: DynPostsRepository,
        metrics: Arc<Mutex<Metrics>>,
        registry: &mut Registry,
        cache_store: Arc<CacheStore>,
    ) -> Self {
        registry.register(
            "post_service_request_counter",
            "Total number of requests to the PostService",
            metrics.lock().await.request_counter.clone(),
        );
        registry.register(
            "post_service_request_duration",
            "Histogram of request durations for the PostService",
            metrics.lock().await.request_duration.clone(),
        );

        Self {
            repository,
            metrics,
            cache_store,
        }
    }

    fn get_tracer(&self) -> BoxedTracer {
        global::tracer("post-service")
    }

    fn inject_trace_context<T>(&self, cx: &Context, request: &mut Request<T>) {
        global::get_text_map_propagator(|propagator| {
            propagator.inject_context(cx, &mut MetadataInjector(request.metadata_mut()))
        });
    }

    fn start_tracing(&self, operation_name: &str, attributes: Vec<KeyValue>) -> TracingContext {
        let start_time = Instant::now();
        let tracer = self.get_tracer();
        let mut span = tracer
            .span_builder(operation_name.to_string())
            .with_kind(SpanKind::Server)
            .with_attributes(attributes)
            .start(&tracer);

        info!("Starting operation: {operation_name}");

        span.add_event(
            "Operation started",
            vec![
                KeyValue::new("operation", operation_name.to_string()),
                KeyValue::new("timestamp", start_time.elapsed().as_secs_f64().to_string()),
            ],
        );

        let cx = Context::current_with_span(span);
        TracingContext { cx, start_time }
    }

    async fn complete_tracing_success(
        &self,
        tracing_ctx: &TracingContext,
        method: Method,
        message: &str,
    ) {
        self.complete_tracing_internal(tracing_ctx, method, true, message)
            .await;
    }

    async fn complete_tracing_error(
        &self,
        tracing_ctx: &TracingContext,
        method: Method,
        error_message: &str,
    ) {
        self.complete_tracing_internal(tracing_ctx, method, false, error_message)
            .await;
    }

    async fn complete_tracing_internal(
        &self,
        tracing_ctx: &TracingContext,
        method: Method,
        is_success: bool,
        message: &str,
    ) {
        let status_str = if is_success { "SUCCESS" } else { "ERROR" };
        let status = if is_success {
            StatusUtils::Success
        } else {
            StatusUtils::Error
        };
        let elapsed = tracing_ctx.start_time.elapsed().as_secs_f64();

        tracing_ctx.cx.span().add_event(
            "Operation completed",
            vec![
                KeyValue::new("status", status_str),
                KeyValue::new("duration_secs", elapsed.to_string()),
                KeyValue::new("message", message.to_string()),
            ],
        );

        if is_success {
            info!("Operation completed successfully: {message}");
        } else {
            error!("Operation failed: {message}");
        }

        self.metrics.lock().await.record(method, status, elapsed);

        tracing_ctx.cx.span().end();
    }
}

#[async_trait]
impl PostsServiceTrait for PostService {
    async fn get_all_posts(
        &self,
        req: FindAllPostRequest,
    ) -> Result<ApiResponsePagination<Vec<PostResponse>>, ErrorResponse> {
        let method = Method::Get;

        let page = req.page.max(1);
        let page_size = req.page_size.max(1);
        let search = if req.search.is_empty() {
            None
        } else {
            Some(req.search.clone())
        };

        let tracing_ctx = self.start_tracing(
            "GetAllPosts",
            vec![
                KeyValue::new("component", "post"),
                KeyValue::new("page", page.to_string()),
                KeyValue::new("page_size", page_size.to_string()),
                KeyValue::new("search", search.clone().unwrap_or_default()),
            ],
        );

        let mut request = Request::new(FindAllPostRequest {
            page,
            page_size,
            search: search.clone().unwrap_or_default(),
        });
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let cache_key = format!(
            "posts:page={page}:size={page_size}:search={}",
            search.clone().unwrap_or_default()
        );

        if let Some(cache) = self
            .cache_store
            .get_from_cache::<ApiResponsePagination<Vec<PostResponse>>>(&cache_key)
            .await
        {
            self.complete_tracing_success(&tracing_ctx, method, "Posts retrieved from cache")
                .await;

            return Ok(cache);
        }

        match self.repository.get_all_posts(page, page_size, search).await {
            Ok((posts, total_items)) => {
                let responses = posts.into_iter().map(PostResponse::from).collect();
                let total_pages = (total_items as f64 / page_size as f64).ceil() as i32;

                let response = ApiResponsePagination {
                    status: "success".to_string(),
                    message: "Posts retrieved successfully".to_string(),
                    data: responses,
                    pagination: Pagination {
                        page,
                        page_size,
                        total_items,
                        total_pages,
                    },
                };

                self.cache_store
                    .set_to_cache(&cache_key, &response, Duration::from_secs(60 * 5))
                    .await;

                self.complete_tracing_success(&tracing_ctx, method, "Posts retrieved successfully")
                    .await;

                Ok(response)
            }
            Err(err) => {
                self.complete_tracing_error(
                    &tracing_ctx,
                    method,
                    &format!("Failed to retrieve posts: {err}"),
                )
                .await;

                Err(ErrorResponse::from(err))
            }
        }
    }
    async fn get_post(
        &self,
        post_id: i32,
    ) -> Result<Option<ApiResponse<PostResponse>>, ErrorResponse> {
        let tracing_ctx = self.start_tracing(
            "GetPost",
            vec![
                KeyValue::new("component", "post"),
                KeyValue::new("id", post_id.to_string()),
            ],
        );

        let cache_key = format!("post:id={post_id}");

        if let Some(cache) = self
            .cache_store
            .get_from_cache::<ApiResponse<PostResponse>>(&cache_key)
            .await
        {
            self.complete_tracing_success(&tracing_ctx, Method::Get, "Post retrieved from cache")
                .await;
            return Ok(Some(cache));
        }

        match self.repository.get_post(post_id).await {
            Ok(Some(post)) => {
                let response = Some(ApiResponse {
                    status: "success".to_string(),
                    message: "Post retrieved successfully".to_string(),
                    data: PostResponse::from(post),
                });

                self.cache_store
                    .set_to_cache(&cache_key, &response.clone(), Duration::from_secs(60 * 5))
                    .await;

                self.complete_tracing_success(
                    &tracing_ctx,
                    Method::Get,
                    "Post retrieved successfully",
                )
                .await;

                Ok(response)
            }
            Ok(None) => {
                self.complete_tracing_error(
                    &tracing_ctx,
                    Method::Get,
                    &format!("Post with id {post_id} not found"),
                )
                .await;

                Err(ErrorResponse::from(AppError::NotFound(format!(
                    "Post with id {post_id} not found",
                ))))
            }
            Err(err) => {
                self.complete_tracing_error(
                    &tracing_ctx,
                    Method::Get,
                    &format!("Error retrieving post: {err}"),
                )
                .await;

                Err(ErrorResponse::from(err))
            }
        }
    }

    async fn get_post_relation(
        &self,
        post_id: i32,
    ) -> Result<ApiResponse<PostRelationResponse>, ErrorResponse> {
        let method = Method::Get;
        let tracing_ctx = self.start_tracing(
            "GetPostRelation",
            vec![
                KeyValue::new("component", "post"),
                KeyValue::new("id", post_id.to_string()),
            ],
        );

        let mut request = Request::new(post_id);
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let cache_key = format!("post_relation:id={post_id}");

        if let Some(cache) = self
            .cache_store
            .get_from_cache::<ApiResponse<PostRelationResponse>>(&cache_key)
            .await
        {
            self.complete_tracing_success(
                &tracing_ctx,
                method,
                "Post relation retrieved from cache",
            )
            .await;

            return Ok(cache);
        }

        match self.repository.get_post_relation(post_id).await {
            Ok(relations) => match relations.into_iter().next() {
                Some(first_relation) => {
                    let response = ApiResponse {
                        status: "success".to_string(),
                        message: "Post relation retrieved successfully".to_string(),
                        data: first_relation,
                    };

                    self.cache_store
                        .set_to_cache(&cache_key, &response.clone(), Duration::from_secs(60 * 5))
                        .await;

                    self.complete_tracing_success(
                        &tracing_ctx,
                        method,
                        "Post relation retrieved successfully",
                    )
                    .await;

                    Ok(response)
                }
                None => {
                    self.complete_tracing_error(&tracing_ctx, method, "Post relation not found")
                        .await;

                    Err(ErrorResponse::from(AppError::NotFound(
                        "Post relation not found".to_string(),
                    )))
                }
            },
            Err(err) => {
                self.complete_tracing_error(
                    &tracing_ctx,
                    method,
                    &format!("Error retrieving post relation: {err}"),
                )
                .await;

                Err(ErrorResponse::from(err))
            }
        }
    }

    async fn create_post(
        &self,
        input: &CreatePostRequest,
    ) -> Result<ApiResponse<PostResponse>, ErrorResponse> {
        let method = Method::Post;
        let tracing_ctx = self.start_tracing(
            "CreatePost",
            vec![
                KeyValue::new("component", "post"),
                KeyValue::new("title", input.title.clone()),
            ],
        );

        let mut request = Request::new(input.clone());
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        match self.repository.create_post(input).await {
            Ok(post) => {
                let response = ApiResponse {
                    status: "success".to_string(),
                    message: "Post created successfully".to_string(),
                    data: PostResponse::from(post),
                };

                self.complete_tracing_success(&tracing_ctx, method, "Post created successfully")
                    .await;

                Ok(response)
            }
            Err(err) => {
                self.complete_tracing_error(
                    &tracing_ctx,
                    method,
                    &format!("Failed to create post: {err}"),
                )
                .await;

                Err(ErrorResponse::from(err))
            }
        }
    }

    async fn update_post(
        &self,
        input: &UpdatePostRequest,
    ) -> Result<ApiResponse<PostResponse>, ErrorResponse> {
        let method = Method::Put;
        let tracing_ctx = self.start_tracing(
            "UpdatePost",
            vec![
                KeyValue::new("component", "post"),
                KeyValue::new("title", input.title.clone()),
            ],
        );

        let mut request = Request::new(input.clone());
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        match self.repository.update_post(input).await {
            Ok(post) => {
                let response = ApiResponse {
                    status: "success".to_string(),
                    message: "Post updated successfully".to_string(),
                    data: PostResponse::from(post),
                };

                let cache_key = format!("post:id={}", input.post_id);
                self.cache_store
                    .set_to_cache(&cache_key, &response.clone(), Duration::from_secs(60 * 5))
                    .await;

                self.complete_tracing_success(&tracing_ctx, method, "Post updated successfully")
                    .await;

                Ok(response)
            }
            Err(err) => {
                self.complete_tracing_error(
                    &tracing_ctx,
                    method,
                    &format!("Failed to update post: {err}"),
                )
                .await;

                Err(ErrorResponse::from(err))
            }
        }
    }

    async fn delete_post(&self, post_id: i32) -> Result<ApiResponse<()>, ErrorResponse> {
        let method = Method::Delete;
        let tracing_ctx = self.start_tracing(
            "DeletePost",
            vec![
                KeyValue::new("component", "post"),
                KeyValue::new("id", post_id.to_string()),
            ],
        );

        let mut request = Request::new(post_id);
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        match self.repository.delete_post(post_id).await {
            Ok(_) => {
                let response = ApiResponse {
                    status: "success".to_string(),
                    message: "Post deleted successfully".to_string(),
                    data: (),
                };

                let cache_key = format!("post:id={post_id}");
                self.cache_store.delete_from_cache(&cache_key).await;

                self.complete_tracing_success(&tracing_ctx, method, "Post deleted successfully")
                    .await;

                Ok(response)
            }
            Err(err) => {
                self.complete_tracing_error(
                    &tracing_ctx,
                    method,
                    &format!("Failed to delete post: {err}"),
                )
                .await;

                Err(ErrorResponse::from(err))
            }
        }
    }
}
