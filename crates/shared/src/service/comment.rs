use crate::{
    abstract_trait::{CommentServiceTrait, DynCommentRepository},
    cache::CacheStore,
    domain::{
        ApiResponse, CommentResponse, CreateCommentRequest, ErrorResponse, UpdateCommentRequest,
    },
    utils::{MetadataInjector, Method, Metrics, Status as StatusUtils, TracingContext},
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
pub struct CommentService {
    repository: DynCommentRepository,
    metrics: Arc<Mutex<Metrics>>,
    cache_store: Arc<CacheStore>,
}

impl std::fmt::Debug for CommentService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CommentService")
            .field("repository", &"DynCommentRepository")
            .finish()
    }
}

impl CommentService {
    pub async fn new(
        repository: DynCommentRepository,
        metrics: Arc<Mutex<Metrics>>,
        registry: &mut Registry,
        cache_store: Arc<CacheStore>,
    ) -> Self {
        registry.register(
            "category_service_request_counter",
            "Total number of requests to the CategoryService",
            metrics.lock().await.request_counter.clone(),
        );
        registry.register(
            "category_service_request_duration",
            "Histogram of request durations for the CategoryService",
            metrics.lock().await.request_duration.clone(),
        );

        Self {
            repository,
            metrics,
            cache_store,
        }
    }

    fn get_tracer(&self) -> BoxedTracer {
        global::tracer("comment-service")
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
impl CommentServiceTrait for CommentService {
    async fn get_comments(&self) -> Result<ApiResponse<Vec<CommentResponse>>, ErrorResponse> {
        let method = Method::Get;

        let tracing_ctx =
            self.start_tracing("GetComments", vec![KeyValue::new("component", "comment")]);

        let mut request = Request::new(());
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let cache_key = "comments".to_string();

        if let Some(cache) = self.cache_store.get_from_cache(&cache_key) {
            self.complete_tracing_success(&tracing_ctx, method, "Comments retrieved from cache")
                .await;
            return Ok(ApiResponse {
                status: "success".to_string(),
                message: "Comments retrieved from cache".to_string(),
                data: cache,
            });
        }

        match self.repository.find_all().await {
            Ok(comments) => {
                let response: Vec<CommentResponse> =
                    comments.into_iter().map(CommentResponse::from).collect();

                self.cache_store.set_to_cache(
                    &cache_key,
                    &response.clone(),
                    Duration::from_secs(60 * 5),
                );

                self.complete_tracing_success(
                    &tracing_ctx,
                    method,
                    "Comments retrieved successfully",
                )
                .await;

                Ok(ApiResponse {
                    status: "success".to_string(),
                    message: "Comments retrieved successfully".to_string(),
                    data: response,
                })
            }
            Err(err) => {
                self.complete_tracing_error(&tracing_ctx, method, "Failed to retrieve comments")
                    .await;

                Err(ErrorResponse::from(err))
            }
        }
    }

    async fn get_comment(
        &self,
        id: i32,
    ) -> Result<Option<ApiResponse<CommentResponse>>, ErrorResponse> {
        let method = Method::Get;

        let tracing_ctx = self.start_tracing(
            "GetComment",
            vec![
                KeyValue::new("component", "category"),
                KeyValue::new("id", id.to_string()),
            ],
        );

        let mut request = Request::new(id);
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let cache_key = format!("comment:id={id}");

        if let Some(cache) = self
            .cache_store
            .get_from_cache::<ApiResponse<CommentResponse>>(&cache_key)
        {
            self.complete_tracing_success(&tracing_ctx, method, "Comment retrieved from cache")
                .await;
            return Ok(Some(cache));
        }

        match self.repository.find_by_id(id).await {
            Ok(comment) => {
                let response = ApiResponse {
                    status: "success".to_string(),
                    message: "Comment retrieved successfully".to_string(),
                    data: CommentResponse::from(comment),
                };

                self.complete_tracing_success(
                    &tracing_ctx,
                    method,
                    "Comment retrieved successfully",
                )
                .await;

                self.cache_store.set_to_cache(
                    &cache_key,
                    &response.clone(),
                    Duration::from_secs(60 * 5),
                );

                Ok(Some(response))
            }
            Err(err) => {
                self.complete_tracing_error(&tracing_ctx, method, "Failed to retrieve comment")
                    .await;

                Err(ErrorResponse::from(err))
            }
        }
    }

    async fn create_comment(
        &self,
        input: &CreateCommentRequest,
    ) -> Result<ApiResponse<CommentResponse>, ErrorResponse> {
        let method = Method::Post;

        let tracing_ctx = self.start_tracing(
            "CreateComment",
            vec![
                KeyValue::new("component", "comment"),
                KeyValue::new("comment.name", input.user_name_comment.clone()),
            ],
        );

        let mut request = Request::new(input.clone());

        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        match self.repository.create(input).await {
            Ok(comment) => {
                self.complete_tracing_success(&tracing_ctx, method, "Comment created successfully")
                    .await;

                Ok(ApiResponse {
                    status: "success".to_string(),
                    message: "Comment created successfully".to_string(),
                    data: CommentResponse::from(comment),
                })
            }
            Err(err) => {
                self.complete_tracing_error(&tracing_ctx, method, "Failed to create comment")
                    .await;

                Err(ErrorResponse::from(err))
            }
        }
    }

    async fn update_comment(
        &self,
        input: &UpdateCommentRequest,
    ) -> Result<Option<ApiResponse<CommentResponse>>, ErrorResponse> {
        let method = Method::Put;

        let tracing_ctx = self.start_tracing(
            "UpdateComment",
            vec![
                KeyValue::new("component", "comment"),
                KeyValue::new("comment.name", input.user_name_comment.clone()),
            ],
        );

        let mut request = Request::new(input.clone());

        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        match self.repository.update(input).await {
            Ok(comment) => {
                self.complete_tracing_success(&tracing_ctx, method, "Comment updated successfully")
                    .await;

                let cache_key = format!("comment:id={}", input.id_post_comment);

                self.cache_store
                    .set_to_cache(&cache_key, &comment, Duration::from_secs(60 * 5));

                Ok(Some(ApiResponse {
                    status: "success".to_string(),
                    message: "Comment updated successfully".to_string(),
                    data: CommentResponse::from(comment),
                }))
            }
            Err(err) => {
                self.complete_tracing_error(&tracing_ctx, method, "Failed to update comment")
                    .await;

                Err(ErrorResponse::from(err))
            }
        }
    }

    async fn delete_comment(&self, id: i32) -> Result<ApiResponse<()>, ErrorResponse> {
        let tracing_ctx = self.start_tracing(
            "DeleteComment",
            vec![
                KeyValue::new("component", "comment"),
                KeyValue::new("id", id.to_string()),
            ],
        );

        let mut request = Request::new(id);
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        match self.repository.delete(id).await {
            Ok(_) => {
                let response = ApiResponse {
                    status: "success".to_string(),
                    message: "Comment deleted successfully".to_string(),
                    data: (),
                };

                let cache_key = format!("comment:id={id}");

                self.cache_store.delete_from_cache(&cache_key);

                self.complete_tracing_success(
                    &tracing_ctx,
                    Method::Delete,
                    "Comment deleted successfully",
                )
                .await;

                Ok(response)
            }
            Err(err) => {
                self.complete_tracing_error(
                    &tracing_ctx,
                    Method::Delete,
                    &format!("Failed to delete comment: {err}"),
                )
                .await;

                Err(ErrorResponse::from(err))
            }
        }
    }
}
