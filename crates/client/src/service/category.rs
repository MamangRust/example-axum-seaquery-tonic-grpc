use async_trait::async_trait;
use genproto::category::{
    CreateCategoryRequest, FindAllCategoryRequest, FindCategoryRequest, UpdateCategoryRequest,
    category_service_client::CategoryServiceClient,
};
use opentelemetry::{
    Context, KeyValue,
    global::{self, BoxedTracer},
    trace::{Span, SpanKind, TraceContextExt, Tracer},
};
use prometheus_client::registry::Registry;
use shared::{
    domain::{
        ApiResponse, ApiResponsePagination, CategoryResponse,
        CreateCategoryRequest as DomainCreateCategoryRequest, ErrorResponse,
        FindAllCategoryRequest as DomainFindAllCategoryRequest,
        UpdateCategoryRequest as DomainUpdateCategoryRequest,
    },
    utils::{MetadataInjector, Method, Metrics, Status as StatusUtils, TracingContext},
};
use std::sync::Arc;
use tokio::{sync::Mutex, time::Instant};
use tonic::{Request, transport::Channel};
use tracing::{error, info};

use crate::abstract_trait::CategoryServiceTrait;

#[derive(Debug)]
pub struct CategoryService {
    client: Arc<Mutex<CategoryServiceClient<Channel>>>,
    metrics: Arc<Mutex<Metrics>>,
}

impl CategoryService {
    pub async fn new(
        client: Arc<Mutex<CategoryServiceClient<Channel>>>,
        metrics: Arc<Mutex<Metrics>>,
        registry: &mut Registry,
    ) -> Self {
        registry.register(
            "category_handler_request_counter",
            "Total number of requests to the AuthService",
            metrics.lock().await.request_counter.clone(),
        );
        registry.register(
            "category_handler_request_duration",
            "Histogram of request durations for the AuthService",
            metrics.lock().await.request_duration.clone(),
        );

        Self { client, metrics }
    }
    fn get_tracer(&self) -> BoxedTracer {
        global::tracer("user-service-client")
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
impl CategoryServiceTrait for CategoryService {
    async fn find_all(
        &self,
        req: &DomainFindAllCategoryRequest,
    ) -> Result<ApiResponsePagination<Vec<CategoryResponse>>, ErrorResponse> {
        let method = Method::Get;
        let tracing_ctx = self.start_tracing(
            "FindAllCategory",
            vec![
                KeyValue::new("component", "category"),
                KeyValue::new("operation", "find_all"),
                KeyValue::new("page", req.page.to_string()),
                KeyValue::new("page_size", req.page_size.to_string()),
                KeyValue::new("search", req.search.clone()),
            ],
        );

        let mut request = Request::new(FindAllCategoryRequest {
            page: req.page,
            page_size: req.page_size,
            search: req.search.clone(),
        });
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        match self.client.lock().await.get_categories(request).await {
            Ok(resp) => {
                let inner = resp.into_inner();
                let response = ApiResponsePagination {
                    status: inner.status,
                    message: inner.message,
                    data: inner.data.into_iter().map(|u| u.into()).collect(),
                    pagination: inner.pagination.unwrap_or_default().into(),
                };

                self.complete_tracing_success(
                    &tracing_ctx,
                    method,
                    &format!(
                        "Categories retrieved successfully (page: {}, size: {})",
                        req.page, req.page_size
                    ),
                )
                .await;

                Ok(response)
            }
            Err(err) => {
                let error_response = ErrorResponse {
                    status: err.code().to_string(),
                    message: err.message().to_string(),
                };

                self.complete_tracing_error(
                    &tracing_ctx,
                    method,
                    &format!(
                        "Failed to retrieve categories (page: {}, size: {}): {}",
                        req.page, req.page_size, error_response.message
                    ),
                )
                .await;

                Err(error_response)
            }
        }
    }

    async fn find_by_id(&self, id: &i32) -> Result<ApiResponse<CategoryResponse>, ErrorResponse> {
        let method = Method::Get;
        let tracing_ctx = self.start_tracing(
            "FindCategoryById",
            vec![
                KeyValue::new("component", "category"),
                KeyValue::new("operation", "find_by_id"),
                KeyValue::new("category.id", *id as i64),
            ],
        );

        let mut request = Request::new(FindCategoryRequest { id: *id });
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        match self.client.lock().await.get_category(request).await {
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
                    &format!("Category {id} found successfully"),
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
                    &format!("Failed to find category {}: {}", id, error_response.message),
                )
                .await;

                Err(error_response)
            }
        }
    }

    async fn create(
        &self,
        req: &DomainCreateCategoryRequest,
    ) -> Result<ApiResponse<CategoryResponse>, ErrorResponse> {
        let method = Method::Post;
        let tracing_ctx = self.start_tracing(
            "CreateCategory",
            vec![
                KeyValue::new("component", "category"),
                KeyValue::new("operation", "create"),
                KeyValue::new("category.name", req.name.clone()),
            ],
        );

        let mut request = Request::new(CreateCategoryRequest {
            name: req.name.clone(),
        });
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        match self.client.lock().await.create_category(request).await {
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
                    &format!("Category '{}' created successfully", req.name),
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
                        "Failed to create category '{}': {}",
                        req.name, error_response.message
                    ),
                )
                .await;

                Err(error_response)
            }
        }
    }

    async fn update(
        &self,
        req: &DomainUpdateCategoryRequest,
    ) -> Result<ApiResponse<CategoryResponse>, ErrorResponse> {
        let method = Method::Put;
        let category_name = req.name.as_deref().unwrap_or("");
        let category_id = req.id.unwrap_or_default();

        let tracing_ctx = self.start_tracing(
            "UpdateCategory",
            vec![
                KeyValue::new("component", "category"),
                KeyValue::new("operation", "update"),
                KeyValue::new("category.id", category_id as i64),
                KeyValue::new("category.name", category_name.to_string()),
            ],
        );

        let mut update_request = UpdateCategoryRequest {
            id: category_id,
            ..Default::default()
        };
        if let Some(name) = &req.name {
            update_request.name = name.clone();
        }

        let mut request = Request::new(update_request);
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        match self.client.lock().await.update_category(request).await {
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
                        "Category updated successfully (ID: {category_id}, Name: {category_name})",
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
                        "Failed to update category (ID: {category_id}, Name: {category_name}): {}",
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
            "DeleteCategory",
            vec![
                KeyValue::new("component", "category"),
                KeyValue::new("operation", "delete"),
                KeyValue::new("category.id", *id as i64),
            ],
        );

        let mut request = Request::new(FindCategoryRequest { id: *id });
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        match self.client.lock().await.delete_category(request).await {
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
                    &format!("Category {id} deleted successfully"),
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
                        "Failed to delete category {}: {}",
                        id, error_response.message
                    ),
                )
                .await;

                Err(error_response)
            }
        }
    }
}
