use async_trait::async_trait;
use genproto::comment::{
    CreateCommentRequest, Empty, FindCommentRequest, UpdateCommentRequest,
    comment_service_client::CommentServiceClient,
};
use opentelemetry::{
    Context, KeyValue,
    global::{self, BoxedTracer},
    trace::{Span, SpanKind, TraceContextExt, Tracer},
};
use prometheus_client::registry::Registry;
use shared::{
    domain::{
        ApiResponse, CommentResponse, CreateCommentRequest as DomainCreateCommentRequest,
        ErrorResponse, UpdateCommentRequest as DomainUpdateCommentRequest,
    },
    utils::{MetadataInjector, Method, Metrics, Status as StatusUtils, TracingContext},
};
use std::sync::Arc;
use tokio::{sync::Mutex, time::Instant};
use tonic::{Request, transport::Channel};
use tracing::{error, info};

use crate::abstract_trait::CommentServiceTrait;

#[derive(Debug)]
pub struct CommentService {
    client: Arc<Mutex<CommentServiceClient<Channel>>>,
    metrics: Arc<Mutex<Metrics>>,
}

impl CommentService {
    pub async fn new(
        client: Arc<Mutex<CommentServiceClient<Channel>>>,
        metrics: Arc<Mutex<Metrics>>,
        registry: &mut Registry,
    ) -> Self {
        registry.register(
            "comment_handler_request_counter",
            "Total number of requests to the CommentService",
            metrics.lock().await.request_counter.clone(),
        );
        registry.register(
            "comment_handler_request_duration",
            "Histogram of request durations for the CommentService",
            metrics.lock().await.request_duration.clone(),
        );

        Self { client, metrics }
    }

    pub fn get_tracer(&self) -> BoxedTracer {
        global::tracer("comment-client-service")
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
    async fn find_all(&self) -> Result<ApiResponse<Vec<CommentResponse>>, ErrorResponse> {
        let method = Method::Get;
        let tracing_ctx = self.start_tracing(
            "FindAllComments",
            vec![
                KeyValue::new("component", "comment"),
                KeyValue::new("operation", "find_all"),
            ],
        );

        let mut request = Request::new(Empty {});
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        match self.client.lock().await.get_comments(request).await {
            Ok(resp) => {
                let inner = resp.into_inner();
                let response = ApiResponse {
                    status: inner.status,
                    message: inner.message,
                    data: inner.data.into_iter().map(Into::into).collect(),
                };

                self.complete_tracing_success(
                    &tracing_ctx,
                    method,
                    "Comments retrieved successfully",
                )
                .await;

                Ok(response)
            }
            Err(status) => {
                let error_response = ErrorResponse {
                    status: status.code().to_string(),
                    message: status.message().to_string(),
                };

                self.complete_tracing_error(
                    &tracing_ctx,
                    method,
                    &format!("Failed to retrieve comments: {}", error_response.message),
                )
                .await;

                Err(error_response)
            }
        }
    }

    async fn find_by_id(&self, id: &i32) -> Result<ApiResponse<CommentResponse>, ErrorResponse> {
        let method = Method::Get;
        let tracing_ctx = self.start_tracing(
            "FindCommentById",
            vec![
                KeyValue::new("component", "comment"),
                KeyValue::new("operation", "find_by_id"),
                KeyValue::new("comment.id", *id as i64),
            ],
        );

        let mut request = Request::new(FindCommentRequest { id: *id });
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        match self.client.lock().await.get_comment(request).await {
            Ok(resp) => {
                let inner = resp.into_inner();
                let response = ApiResponse {
                    status: inner.status,
                    message: inner.message,
                    data: inner.data.into(),
                };

                self.complete_tracing_success(
                    &tracing_ctx,
                    method,
                    &format!("Comment with id {id} retrieved successfully"),
                )
                .await;

                Ok(response)
            }
            Err(status) => {
                let error_response = ErrorResponse {
                    status: status.code().to_string(),
                    message: status.message().to_string(),
                };

                self.complete_tracing_error(
                    &tracing_ctx,
                    method,
                    &format!(
                        "Failed to retrieve comment with id {}: {}",
                        id, error_response.message
                    ),
                )
                .await;

                Err(error_response)
            }
        }
    }

    async fn create(
        &self,
        req: &DomainCreateCommentRequest,
    ) -> Result<ApiResponse<CommentResponse>, ErrorResponse> {
        let method = Method::Post;
        let tracing_ctx = self.start_tracing(
            "CreateComment",
            vec![
                KeyValue::new("component", "comment"),
                KeyValue::new("operation", "create"),
                KeyValue::new("comment.post_id", req.id_post_comment as i64),
                KeyValue::new("comment.user", req.user_name_comment.clone()),
            ],
        );

        let mut request = Request::new(CreateCommentRequest {
            id_post_comment: req.id_post_comment,
            user_name_comment: req.user_name_comment.clone(),
            comment: req.comment.clone(),
        });
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        match self.client.lock().await.create_comment(request).await {
            Ok(resp) => {
                let inner = resp.into_inner();
                let response = ApiResponse {
                    status: inner.status,
                    message: inner.message,
                    data: inner.data.into(),
                };

                self.complete_tracing_success(
                    &tracing_ctx,
                    method,
                    &format!(
                        "Comment created successfully for post {} by user {}",
                        req.id_post_comment, req.user_name_comment
                    ),
                )
                .await;

                Ok(response)
            }
            Err(status) => {
                let error_response = ErrorResponse {
                    status: status.code().to_string(),
                    message: status.message().to_string(),
                };

                self.complete_tracing_error(
                    &tracing_ctx,
                    method,
                    &format!(
                        "Failed to create comment for post {}: {}",
                        req.id_post_comment, error_response.message
                    ),
                )
                .await;

                Err(error_response)
            }
        }
    }

    async fn update(
        &self,
        req: &DomainUpdateCommentRequest,
    ) -> Result<ApiResponse<CommentResponse>, ErrorResponse> {
        let method = Method::Put;
        let tracing_ctx = self.start_tracing(
            "UpdateComment",
            vec![
                KeyValue::new("component", "comment"),
                KeyValue::new("operation", "update"),
                KeyValue::new("comment.id", req.id_post_comment.unwrap_or_default() as i64),
                KeyValue::new("comment.user", req.user_name_comment.clone()),
            ],
        );

        let mut request = Request::new(UpdateCommentRequest {
            id_post_comment: req.id_post_comment.unwrap_or_default(),
            user_name_comment: req.user_name_comment.clone(),
            comment: req.comment.clone(),
        });
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        match self.client.lock().await.update_comment(request).await {
            Ok(resp) => {
                let inner = resp.into_inner();
                let response = ApiResponse {
                    status: inner.status,
                    message: inner.message,
                    data: inner.data.into(),
                };

                self.complete_tracing_success(
                    &tracing_ctx,
                    method,
                    &format!(
                        "Comment {} updated successfully by user {}",
                        req.id_post_comment.unwrap_or_default(),
                        req.user_name_comment
                    ),
                )
                .await;

                Ok(response)
            }
            Err(status) => {
                let error_response = ErrorResponse {
                    status: status.code().to_string(),
                    message: status.message().to_string(),
                };

                self.complete_tracing_error(
                    &tracing_ctx,
                    method,
                    &format!(
                        "Failed to update comment {}: {}",
                        req.id_post_comment.unwrap_or_default(),
                        error_response.message
                    ),
                )
                .await;

                Err(error_response)
            }
        }
    }

    async fn delete(&self, id: &i32) -> Result<ApiResponse<()>, ErrorResponse> {
        let method = Method::Delete;
        let tracing_ctx = self.start_tracing(
            "DeleteComment",
            vec![
                KeyValue::new("component", "comment"),
                KeyValue::new("operation", "delete"),
                KeyValue::new("comment.id", *id as i64),
            ],
        );

        let mut request = Request::new(FindCommentRequest { id: *id });
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        match self.client.lock().await.delete_comment(request).await {
            Ok(resp) => {
                let inner = resp.into_inner();
                let response = ApiResponse {
                    status: inner.status,
                    message: inner.message,
                    data: (),
                };

                self.complete_tracing_success(
                    &tracing_ctx,
                    method,
                    &format!("Comment {id} deleted successfully"),
                )
                .await;

                Ok(response)
            }
            Err(status) => {
                let error_response = ErrorResponse {
                    status: status.code().to_string(),
                    message: status.message().to_string(),
                };

                self.complete_tracing_error(
                    &tracing_ctx,
                    method,
                    &format!(
                        "Failed to delete comment {}: {}",
                        id, error_response.message
                    ),
                )
                .await;

                Err(error_response)
            }
        }
    }
}
