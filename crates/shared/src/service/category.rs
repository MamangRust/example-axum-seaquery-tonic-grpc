use crate::{
    abstract_trait::{CategoryServiceTrait, DynCategoryRepository},
    cache::CacheStore,
    domain::{
        ApiResponse, ApiResponsePagination, CategoryResponse, CreateCategoryRequest, ErrorResponse,
        FindAllCategoryRequest, Pagination, UpdateCategoryRequest,
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
pub struct CategoryService {
    repository: DynCategoryRepository,
    metrics: Arc<Mutex<Metrics>>,
    cache_store: Arc<CacheStore>,
}

impl std::fmt::Debug for CategoryService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CategoryService")
            .field("repository", &"DynCategoryRepository")
            .finish()
    }
}

impl CategoryService {
    pub async fn new(
        repository: DynCategoryRepository,
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
        global::tracer("category-service")
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
    async fn get_categories(
        &self,
        req: FindAllCategoryRequest,
    ) -> Result<ApiResponsePagination<Vec<CategoryResponse>>, ErrorResponse> {
        let method = Method::Get;

        let page = if req.page > 0 { req.page } else { 1 };
        let page_size = if req.page_size > 0 { req.page_size } else { 10 };
        let search = if req.search.is_empty() {
            None
        } else {
            Some(req.search.clone())
        };

        let tracing_ctx = self.start_tracing(
            "GetCategories",
            vec![
                KeyValue::new("component", "category"),
                KeyValue::new("page", page.to_string()),
                KeyValue::new("page_size", page_size.to_string()),
                KeyValue::new("search", search.clone().unwrap_or_default()),
            ],
        );

        let mut request = Request::new(FindAllCategoryRequest {
            page,
            page_size,
            search: search.clone().unwrap_or_default(),
        });

        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let cache_key = format!(
            "categories:page={page}:size={page_size}:search={}",
            search.clone().unwrap_or_default()
        );

        if let Some(cached) = self
            .cache_store
            .get_from_cache::<ApiResponsePagination<Vec<CategoryResponse>>>(&cache_key)
        {
            info!("Found categories in cache");

            self.complete_tracing_success(&tracing_ctx, method, "Categories retrieved from cache")
                .await;

            return Ok(cached);
        }

        match self.repository.find_all(page, page_size, search).await {
            Ok((categories, total_items)) => {
                let total_pages = (total_items as f64 / page_size as f64).ceil() as i32;
                let category_responses = categories
                    .into_iter()
                    .map(CategoryResponse::from)
                    .collect::<Vec<_>>();

                let response = ApiResponsePagination {
                    status: "success".to_string(),
                    message: "Categories retrieved successfully".to_string(),
                    data: category_responses.clone(),
                    pagination: Pagination {
                        page,
                        page_size,
                        total_items,
                        total_pages,
                    },
                };

                self.cache_store
                    .set_to_cache(&cache_key, &response, Duration::from_secs(60 * 5));

                self.complete_tracing_success(
                    &tracing_ctx,
                    method,
                    "Categories retrieved from database",
                )
                .await;

                Ok(response)
            }

            Err(err) => {
                self.complete_tracing_error(
                    &tracing_ctx,
                    method,
                    &format!("Failed to retrieve categories: {err}"),
                )
                .await;

                Err(ErrorResponse::from(err))
            }
        }
    }

    async fn get_category(
        &self,
        id: i32,
    ) -> Result<Option<ApiResponse<CategoryResponse>>, ErrorResponse> {
        let method = Method::Get;

        let tracing_ctx = self.start_tracing(
            "GetCategory",
            vec![
                KeyValue::new("component", "category"),
                KeyValue::new("id", id.to_string()),
            ],
        );

        let mut request = Request::new(id);
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let cache_key = format!("category:id={id}");

        if let Some(cached) = self
            .cache_store
            .get_from_cache::<ApiResponse<CategoryResponse>>(&cache_key)
        {
            info!("Found category in cache");

            self.complete_tracing_success(&tracing_ctx, method, "Category retrieved from cache")
                .await;

            return Ok(Some(cached));
        }

        match self.repository.find_by_id(id).await {
            Ok(Some(category)) => {
                let response = ApiResponse {
                    status: "success".to_string(),
                    message: "Category retrieved successfully".to_string(),
                    data: CategoryResponse::from(category),
                };

                self.cache_store
                    .set_to_cache(&cache_key, &response, Duration::from_secs(60 * 5));

                self.complete_tracing_success(
                    &tracing_ctx,
                    method,
                    "Category retrieved successfully",
                )
                .await;

                Ok(Some(response))
            }
            Ok(None) => {
                self.complete_tracing_error(&tracing_ctx, method, "Category not found")
                    .await;

                Ok(None)
            }
            Err(err) => {
                self.complete_tracing_error(
                    &tracing_ctx,
                    method,
                    &format!("Error retrieving category: {err}"),
                )
                .await;

                Err(ErrorResponse::from(err))
            }
        }
    }

    async fn create_category(
        &self,
        input: &CreateCategoryRequest,
    ) -> Result<ApiResponse<CategoryResponse>, ErrorResponse> {
        let method = Method::Post;

        let tracing_ctx = self.start_tracing(
            "CreateCategory",
            vec![
                KeyValue::new("component", "category"),
                KeyValue::new("category.name", input.name.clone()),
            ],
        );

        let mut request = Request::new(input.clone());
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        match self.repository.create(input).await {
            Ok(category) => {
                let response = ApiResponse {
                    status: "success".to_string(),
                    message: "Category created successfully".to_string(),
                    data: CategoryResponse::from(category),
                };

                self.complete_tracing_success(
                    &tracing_ctx,
                    method,
                    "Category created successfully",
                )
                .await;

                Ok(response)
            }
            Err(err) => {
                self.complete_tracing_error(
                    &tracing_ctx,
                    method,
                    &format!("Category creation failed: {err}"),
                )
                .await;

                Err(ErrorResponse::from(err))
            }
        }
    }

    async fn update_category(
        &self,
        input: &UpdateCategoryRequest,
    ) -> Result<Option<ApiResponse<CategoryResponse>>, ErrorResponse> {
        let method = Method::Put;

        let tracing_ctx = self.start_tracing(
            "UpdateCategory",
            vec![
                KeyValue::new("component", "category"),
                KeyValue::new("category.name", input.name.clone()),
            ],
        );

        let mut request = Request::new(input.clone());
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        match self.repository.update(input).await {
            Ok(category) => {
                let response = Some(ApiResponse {
                    status: "success".to_string(),
                    message: "Category updated successfully".to_string(),
                    data: CategoryResponse::from(category),
                });

                let cache_key = format!("category:id={}", input.id);

                self.cache_store
                    .set_to_cache(&cache_key, &response, Duration::from_secs(60 * 5));

                self.complete_tracing_success(
                    &tracing_ctx,
                    method,
                    "Category updated successfully",
                )
                .await;

                Ok(response)
            }
            Err(err) => {
                self.complete_tracing_error(
                    &tracing_ctx,
                    method,
                    &format!("Category update failed: {err}"),
                )
                .await;

                Err(ErrorResponse::from(err))
            }
        }
    }

    async fn delete_category(&self, id: i32) -> Result<ApiResponse<()>, ErrorResponse> {
        let method = Method::Delete;
        let tracing_ctx = self.start_tracing(
            "DeleteCategory",
            vec![
                KeyValue::new("component", "category"),
                KeyValue::new("id", id.to_string()),
            ],
        );

        let mut request = Request::new(id);
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        match self.repository.delete(id).await {
            Ok(_) => {
                let response = ApiResponse {
                    status: "success".to_string(),
                    message: "Category deleted successfully".to_string(),
                    data: (),
                };

                let cache_key = format!("category:id={id}");

                self.cache_store.delete_from_cache(&cache_key);

                self.complete_tracing_success(
                    &tracing_ctx,
                    method,
                    "Category deleted successfully",
                )
                .await;

                Ok(response)
            }
            Err(err) => {
                self.complete_tracing_error(
                    &tracing_ctx,
                    method,
                    &format!("Failed to delete category: {err}"),
                )
                .await;

                Err(ErrorResponse::from(err))
            }
        }
    }
}
