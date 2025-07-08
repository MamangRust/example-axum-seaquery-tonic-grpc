use async_trait::async_trait;
use genproto::post::{
    CreatePostRequest, FindAllPostRequest, FindPostRequest, UpdatePostRequest,
    posts_service_client::PostsServiceClient,
};
use opentelemetry::{
    Context, KeyValue,
    global::{self, BoxedTracer},
    trace::{Span, SpanKind, TraceContextExt, Tracer},
};
use prometheus_client::registry::Registry;
use shared::{
    domain::{
        ApiResponse, ApiResponsePagination, CreatePostRequest as DomainCreatePostRequest,
        ErrorResponse, FindAllPostRequest as DomainFindAllPostRequest, PostRelationResponse,
        PostResponse, UpdatePostRequest as DomainUpdatePostRequest,
    },
    utils::{MetadataInjector, Method, Metrics, Status as StatusUtils, TracingContext},
};
use std::sync::Arc;
use tokio::{sync::Mutex, time::Instant};
use tonic::{Request, transport::Channel};
use tracing::{error, info};

use crate::abstract_trait::PostsServiceTrait;

#[derive(Debug)]
pub struct PostsService {
    client: Arc<Mutex<PostsServiceClient<Channel>>>,
    metrics: Arc<Mutex<Metrics>>,
}

impl PostsService {
    pub async fn new(
        client: Arc<Mutex<PostsServiceClient<Channel>>>,
        metrics: Arc<Mutex<Metrics>>,
        registry: &mut Registry,
    ) -> Self {
        registry.register(
            "post_handler_request_counter",
            "Total number of requests to the PostService",
            metrics.lock().await.request_counter.clone(),
        );
        registry.register(
            "post_handler_request_duration",
            "Histogram of request durations for the PostService",
            metrics.lock().await.request_duration.clone(),
        );

        Self { client, metrics }
    }

    fn get_tracer(&self) -> BoxedTracer {
        global::tracer("post-service-client")
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
impl PostsServiceTrait for PostsService {
    async fn find_all(
        &self,
        req: &DomainFindAllPostRequest,
    ) -> Result<ApiResponsePagination<Vec<PostResponse>>, ErrorResponse> {
        let method = Method::Get;
        let tracing_ctx = self.start_tracing(
            "FindAllPost",
            vec![
                KeyValue::new("component", "post"),
                KeyValue::new("operation", "find_all"),
                KeyValue::new("page", req.page.to_string()),
                KeyValue::new("page_size", req.page_size.to_string()),
                KeyValue::new("search", req.search.clone()),
            ],
        );

        let mut request = Request::new(FindAllPostRequest {
            page: req.page,
            page_size: req.page_size,
            search: req.search.clone(),
        });
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        match self.client.lock().await.find_all_posts(request).await {
            Ok(resp) => {
                let inner = resp.into_inner();
                let response = ApiResponsePagination {
                    status: inner.status,
                    message: inner.message,
                    data: inner.data.into_iter().map(Into::into).collect(),
                    pagination: inner.pagination.unwrap_or_default().into(),
                };

                self.complete_tracing_success(&tracing_ctx, method, "Posts retrieved successfully")
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
                    &format!("Failed to retrieve posts: {}", error_response.message),
                )
                .await;

                Err(error_response)
            }
        }
    }

    async fn find_relation(
        &self,
        id: &i32,
    ) -> Result<ApiResponse<PostRelationResponse>, ErrorResponse> {
        let method = Method::Get;
        let tracing_ctx = self.start_tracing(
            "FindPostRelation",
            vec![
                KeyValue::new("component", "post"),
                KeyValue::new("operation", "find_relation"),
                KeyValue::new("post.id", id.to_string()),
            ],
        );

        let mut request = Request::new(FindPostRequest { post_id: *id });
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        match self.client.lock().await.find_post_relation(request).await {
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
                    "Post relation retrieved successfully",
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
                        "Failed to retrieve post relation: {}",
                        error_response.message
                    ),
                )
                .await;

                Err(error_response)
            }
        }
    }

    async fn find_by_id(&self, id: &i32) -> Result<ApiResponse<PostResponse>, ErrorResponse> {
        let method = Method::Get;
        let tracing_ctx = self.start_tracing(
            "FindPostById",
            vec![
                KeyValue::new("component", "post"),
                KeyValue::new("operation", "find_by_id"),
                KeyValue::new("post.id", id.to_string()),
            ],
        );

        let mut request = Request::new(FindPostRequest { post_id: *id });
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        match self.client.lock().await.find_post(request).await {
            Ok(resp) => {
                let inner = resp.into_inner();
                let response = ApiResponse {
                    status: inner.status,
                    message: inner.message,
                    data: inner.data.into(),
                };

                self.complete_tracing_success(&tracing_ctx, method, "Post retrieved successfully")
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
                    &format!("Failed to retrieve post: {}", error_response.message),
                )
                .await;

                Err(error_response)
            }
        }
    }

    async fn create(
        &self,
        req: &DomainCreatePostRequest,
    ) -> Result<ApiResponse<PostResponse>, ErrorResponse> {
        let method = Method::Post;
        let tracing_ctx = self.start_tracing(
            "CreatePost",
            vec![
                KeyValue::new("component", "post"),
                KeyValue::new("operation", "create"),
                KeyValue::new("post.title", req.title.clone()),
                KeyValue::new("post.category_id", req.category_id.to_string()),
                KeyValue::new("post.user_id", req.user_id.to_string()),
            ],
        );

        let mut request = Request::new(CreatePostRequest {
            title: req.title.clone(),
            body: req.body.clone(),
            file: req.file.clone(),
            category_id: req.category_id,
            user_id: req.user_id,
            user_name: req.user_name.clone(),
        });
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        match self.client.lock().await.create_post(request).await {
            Ok(resp) => {
                let inner = resp.into_inner();
                let response = ApiResponse {
                    status: inner.status,
                    message: inner.message,
                    data: inner.data.into(),
                };

                self.complete_tracing_success(&tracing_ctx, method, "Post created successfully")
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
                    &format!("Failed to create post: {}", error_response.message),
                )
                .await;

                Err(error_response)
            }
        }
    }
    async fn update(
        &self,
        req: &DomainUpdatePostRequest,
    ) -> Result<ApiResponse<PostResponse>, ErrorResponse> {
        let method = Method::Put;
        let tracing_ctx = self.start_tracing(
            "UpdatePost",
            vec![
                KeyValue::new("component", "post"),
                KeyValue::new("operation", "update"),
                KeyValue::new("post.id", req.post_id as i64),
                KeyValue::new("post.title", req.title.clone()),
                KeyValue::new("post.category_id", req.category_id.to_string()),
                KeyValue::new("post.user_id", req.user_id.to_string()),
            ],
        );

        let mut request = Request::new(UpdatePostRequest {
            post_id: req.post_id,
            title: req.title.clone(),
            body: req.body.clone(),
            file: req.file.clone(),
            category_id: req.category_id,
            user_id: req.user_id,
            user_name: req.user_name.clone(),
        });
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        match self.client.lock().await.update_post(request).await {
            Ok(resp) => {
                let inner = resp.into_inner();
                let response = ApiResponse {
                    status: inner.status,
                    message: inner.message,
                    data: inner.data.into(),
                };

                self.complete_tracing_success(&tracing_ctx, method, "Post updated successfully")
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
                    &format!("Failed to update post: {}", error_response.message),
                )
                .await;

                Err(error_response)
            }
        }
    }

    async fn delete(&self, id: &i32) -> Result<ApiResponse<()>, ErrorResponse> {
        let method = Method::Delete;
        let tracing_ctx = self.start_tracing(
            "DeletePost",
            vec![
                KeyValue::new("component", "post"),
                KeyValue::new("operation", "delete"),
                KeyValue::new("post.id", id.to_string()),
            ],
        );

        let mut request = Request::new(FindPostRequest { post_id: *id });
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        match self.client.lock().await.delete_post(request).await {
            Ok(resp) => {
                let inner = resp.into_inner();
                let response = ApiResponse {
                    status: inner.status,
                    message: inner.message,
                    data: (),
                };

                self.complete_tracing_success(&tracing_ctx, method, "Post deleted successfully")
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
                    &format!("Failed to delete post: {}", error_response.message),
                )
                .await;

                Err(error_response)
            }
        }
    }
}
