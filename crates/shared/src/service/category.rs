use crate::{
    abstract_trait::{CategoryServiceTrait, DynCategoryRepository},
    domain::{
        ApiResponse, ApiResponsePagination, CategoryResponse, CreateCategoryRequest, ErrorResponse,
        FindAllCategoryRequest, Pagination, UpdateCategoryRequest,
    },
    utils::{AppError, MetadataInjector, Method, Metrics},
};
use async_trait::async_trait;
use opentelemetry::{
    Context, KeyValue,
    global::{self, BoxedTracer},
    trace::{SpanKind, TraceContextExt, Tracer},
};
use std::sync::Arc;
use tokio::sync::Mutex;
use tonic::{Request, Status};
use tracing::info;

#[derive(Clone)]
pub struct CategoryService {
    repository: DynCategoryRepository,
    metrics: Arc<Mutex<Metrics>>,
}

impl std::fmt::Debug for CategoryService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CategoryService")
            .field("repository", &"DynCategoryRepository")
            .finish()
    }
}

impl CategoryService {
    pub fn new(repository: DynCategoryRepository, metrics: Arc<Mutex<Metrics>>) -> Self {
        Self {
            repository,
            metrics,
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

    fn add_completion_event<T>(
        &self,
        cx: &Context,
        result: &Result<T, Status>,
        event_name: String,
    ) {
        let status = match result {
            Ok(_) => "OK",
            Err(status) => match status.code() {
                tonic::Code::Ok => "OK",
                tonic::Code::NotFound => "NOT_FOUND",
                tonic::Code::InvalidArgument => "INVALID_ARGUMENT",
                tonic::Code::Internal => "INTERNAL_ERROR",
                _ => "UNKNOWN_ERROR",
            },
        };

        cx.span()
            .add_event(event_name, vec![KeyValue::new("status", status)]);
    }
}

#[async_trait]
impl CategoryServiceTrait for CategoryService {
    async fn get_categories(
        &self,
        req: FindAllCategoryRequest,
    ) -> Result<ApiResponsePagination<Vec<CategoryResponse>>, ErrorResponse> {
        self.metrics.lock().await.inc_requests(Method::Get);

        let tracer = self.get_tracer();

        let page = if req.page > 0 { req.page } else { 1 };
        let page_size = if req.page_size > 0 { req.page_size } else { 10 };
        let search = if req.search.is_empty() {
            None
        } else {
            Some(req.search.clone())
        };

        let span = tracer
            .span_builder("GetCategories")
            .with_kind(SpanKind::Server)
            .with_attributes([
                KeyValue::new("component", "category"),
                KeyValue::new("page", page.to_string()),
                KeyValue::new("page_size", page_size.to_string()),
                KeyValue::new("search", search.clone().unwrap_or("".to_string())),
            ])
            .start(&tracer);
        let cx = Context::current_with_span(span);

        let mut request = Request::new(FindAllCategoryRequest {
            page,
            page_size,
            search: search.clone().unwrap_or("".to_string()),
        });

        self.inject_trace_context(&cx, &mut request);

        let (categories, total_items) = self
            .repository
            .find_all(page, page_size, search)
            .await
            .map_err(ErrorResponse::from)?;

        info!("Found {} categories", categories.len());

        let total_pages = (total_items as f64 / page_size as f64).ceil() as i32;

        let category_responses: Vec<CategoryResponse> =
            categories.into_iter().map(CategoryResponse::from).collect();

        self.add_completion_event(
            &cx,
            &Ok(()),
            "Categories retrieved successfully".to_string(),
        );

        Ok(ApiResponsePagination {
            status: "success".to_string(),
            message: "Categories retrieved successfully".to_string(),
            data: category_responses,
            pagination: Pagination {
                page,
                page_size,
                total_items,
                total_pages,
            },
        })
    }

    async fn get_category(
        &self,
        id: i32,
    ) -> Result<Option<ApiResponse<CategoryResponse>>, ErrorResponse> {
        self.metrics.lock().await.inc_requests(Method::Get);

        let tracer = self.get_tracer();

        let span = tracer
            .span_builder("GetCategory")
            .with_kind(SpanKind::Server)
            .with_attributes([
                KeyValue::new("component", "category"),
                KeyValue::new("id", id.to_string()),
            ])
            .start(&tracer);
        let cx = Context::current_with_span(span);

        let mut request = Request::new(id);

        self.inject_trace_context(&cx, &mut request);

        let category = self
            .repository
            .find_by_id(id)
            .await
            .map_err(ErrorResponse::from)?;

        self.add_completion_event(&cx, &Ok(()), "Category retrieved successfully".to_string());

        if let Some(category) = category {
            Ok(Some(ApiResponse {
                status: "success".to_string(),
                message: "Category retrieved successfully".to_string(),
                data: CategoryResponse::from(category),
            }))
        } else {
            Err(ErrorResponse::from(AppError::NotFound(format!(
                "Category with id {id} not found",
            ))))
        }
    }

    async fn create_category(
        &self,
        input: &CreateCategoryRequest,
    ) -> Result<ApiResponse<CategoryResponse>, ErrorResponse> {
        self.metrics.lock().await.inc_requests(Method::Post);

        let tracer = self.get_tracer();

        let span = tracer
            .span_builder("CreateCategory")
            .with_kind(SpanKind::Server)
            .with_attributes([
                KeyValue::new("component", "category"),
                KeyValue::new("category.name", input.name.clone()),
            ])
            .start(&tracer);
        let cx = Context::current_with_span(span);

        let mut request = Request::new(input.clone());

        self.inject_trace_context(&cx, &mut request);

        self.add_completion_event(&cx, &Ok(()), "Category created successfully".to_string());

        let category = self
            .repository
            .create(input)
            .await
            .map_err(ErrorResponse::from)?;

        info!("Category created: {:#?}", category);

        Ok(ApiResponse {
            status: "success".to_string(),
            message: "Category created successfully".to_string(),
            data: CategoryResponse::from(category),
        })
    }

    async fn update_category(
        &self,
        input: &UpdateCategoryRequest,
    ) -> Result<Option<ApiResponse<CategoryResponse>>, ErrorResponse> {
        self.metrics.lock().await.inc_requests(Method::Put);

        let tracer = self.get_tracer();

        let span = tracer
            .span_builder("UpdateCategory")
            .with_kind(SpanKind::Server)
            .with_attributes([
                KeyValue::new("component", "category"),
                KeyValue::new(
                    "category.name",
                    input.name.clone().unwrap_or("".to_string()),
                ),
            ])
            .start(&tracer);
        let cx = Context::current_with_span(span);

        let mut request = Request::new(input.clone());

        self.inject_trace_context(&cx, &mut request);

        let category = self
            .repository
            .update(input)
            .await
            .map_err(ErrorResponse::from)?;

        self.add_completion_event(&cx, &Ok(()), "Category updated successfully".to_string());

        Ok(Some(ApiResponse {
            status: "success".to_string(),
            message: "Category updated successfully".to_string(),
            data: CategoryResponse::from(category),
        }))
    }

    async fn delete_category(&self, id: i32) -> Result<ApiResponse<()>, ErrorResponse> {
        self.metrics.lock().await.inc_requests(Method::Delete);

        let tracer = self.get_tracer();

        let span = tracer
            .span_builder("DeleteCategory")
            .with_kind(SpanKind::Server)
            .with_attributes([
                KeyValue::new("component", "category"),
                KeyValue::new("id", id.to_string()),
            ])
            .start(&tracer);
        let cx = Context::current_with_span(span);

        let mut request = Request::new(id);

        self.inject_trace_context(&cx, &mut request);

        self.repository
            .delete(id)
            .await
            .map_err(ErrorResponse::from)?;

        self.add_completion_event(&cx, &Ok(()), "Category deleted successfully".to_string());

        Ok(ApiResponse {
            status: "success".to_string(),
            message: "Category deleted successfully".to_string(),
            data: (),
        })
    }
}
