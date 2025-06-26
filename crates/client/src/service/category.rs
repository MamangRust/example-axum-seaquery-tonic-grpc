use async_trait::async_trait;
use genproto::category::{
    CreateCategoryRequest, FindAllCategoryRequest, FindCategoryRequest, UpdateCategoryRequest,
    category_service_client::CategoryServiceClient,
};
use opentelemetry::{
    Context, KeyValue,
    global::{self, BoxedTracer},
    trace::{SpanKind, TraceContextExt, Tracer},
};
use shared::{
    domain::{
        ApiResponse, ApiResponsePagination, CategoryResponse,
        CreateCategoryRequest as DomainCreateCategoryRequest, ErrorResponse,
        FindAllCategoryRequest as DomainFindAllCategoryRequest,
        UpdateCategoryRequest as DomainUpdateCategoryRequest,
    },
    utils::{MetadataInjector, Method, Metrics},
};
use std::sync::Arc;
use tokio::sync::Mutex;
use tonic::{Request, Status, transport::Channel};

use crate::abstract_trait::CategoryServiceTrait;

#[derive(Debug)]
pub struct CategoryService {
    client: Arc<Mutex<CategoryServiceClient<Channel>>>,
    metrics: Arc<Mutex<Metrics>>,
}

impl CategoryService {
    pub fn new(
        client: Arc<Mutex<CategoryServiceClient<Channel>>>,
        metrics: Arc<Mutex<Metrics>>,
    ) -> Self {
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

    fn add_completion_event<T>(
        &self,
        cx: &Context,
        result: &Result<T, Status>,
        event_name: String,
    ) {
        let status = match result {
            Ok(_) => "OK".to_string(),
            Err(status) => status.code().to_string(),
        };

        cx.span()
            .add_event(event_name, vec![KeyValue::new("status", status)]);
    }
}

#[async_trait]
impl CategoryServiceTrait for CategoryService {
    async fn find_all(
        &self,
        req: &DomainFindAllCategoryRequest,
    ) -> Result<ApiResponsePagination<Vec<CategoryResponse>>, ErrorResponse> {
        self.metrics.lock().await.inc_requests(Method::Get);

        let tracer = self.get_tracer();
        let span = tracer
            .span_builder("FindAllCategory")
            .with_kind(SpanKind::Server)
            .with_attributes([
                KeyValue::new("component", "category"),
                KeyValue::new("page", req.page.to_string()),
                KeyValue::new("page_size", req.page_size.to_string()),
                KeyValue::new("search", req.search.clone()),
            ])
            .start(&tracer);

        let cx = Context::current_with_span(span);

        let mut request = Request::new(FindAllCategoryRequest {
            page: req.page,
            page_size: req.page_size,
            search: req.search.clone(),
        });
        self.inject_trace_context(&cx, &mut request);

        let response = self.client.lock().await.get_categories(request).await;

        self.add_completion_event(&cx, &response, "find_all_category_response".to_string());

        response
            .map(|resp| {
                let inner = resp.into_inner();
                ApiResponsePagination {
                    status: inner.status,
                    message: inner.message,
                    data: inner.data.into_iter().map(|u| u.into()).collect(),
                    pagination: inner.pagination.unwrap_or_default().into(),
                }
            })
            .map_err(|status| ErrorResponse {
                status: status.code().to_string(),
                message: status.message().to_string(),
            })
    }

    async fn find_by_id(&self, id: &i32) -> Result<ApiResponse<CategoryResponse>, ErrorResponse> {
        self.metrics.lock().await.inc_requests(Method::Get);

        let tracer = self.get_tracer();
        let span = tracer
            .span_builder("FindCategoryById")
            .with_kind(SpanKind::Server)
            .with_attributes([
                KeyValue::new("component", "category"),
                KeyValue::new("category.id", id.to_string()),
            ])
            .start(&tracer);
        let cx = Context::current_with_span(span);

        let mut request = Request::new(FindCategoryRequest { id: *id });
        self.inject_trace_context(&cx, &mut request);

        let response = self.client.lock().await.get_category(request).await;

        self.add_completion_event(&cx, &response, "find_category_by_id_response".to_string());

        response
            .map(|resp| {
                let inner = resp.into_inner();
                ApiResponse {
                    status: inner.status,
                    message: inner.message,
                    data: inner.data.into(),
                }
            })
            .map_err(|status| ErrorResponse {
                status: status.code().to_string(),
                message: status.message().to_string(),
            })
    }

    async fn create(
        &self,
        req: &DomainCreateCategoryRequest,
    ) -> Result<ApiResponse<CategoryResponse>, ErrorResponse> {
        self.metrics.lock().await.inc_requests(Method::Post);

        let tracer = self.get_tracer();
        let span = tracer
            .span_builder("CreateCategory")
            .with_kind(SpanKind::Server)
            .with_attributes([
                KeyValue::new("component", "category"),
                KeyValue::new("category.name", req.name.clone()),
            ])
            .start(&tracer);
        let cx = Context::current_with_span(span);

        let mut request = Request::new(CreateCategoryRequest {
            name: req.name.clone(),
        });
        self.inject_trace_context(&cx, &mut request);

        let response = self.client.lock().await.create_category(request).await;

        self.add_completion_event(&cx, &response, "create_category_response".to_string());

        response
            .map(|resp| {
                let inner = resp.into_inner();
                ApiResponse {
                    status: inner.status,
                    message: inner.message,
                    data: inner.data.into(),
                }
            })
            .map_err(|status| ErrorResponse {
                status: status.code().to_string(),
                message: status.message().to_string(),
            })
    }

    async fn update(
        &self,
        req: &DomainUpdateCategoryRequest,
    ) -> Result<ApiResponse<CategoryResponse>, ErrorResponse> {
        self.metrics.lock().await.inc_requests(Method::Put);

        let tracer = self.get_tracer();
        let span = tracer
            .span_builder("UpdateCategory")
            .with_kind(SpanKind::Server)
            .with_attributes([
                KeyValue::new("component", "category"),
                KeyValue::new(
                    "category.name",
                    req.name.as_ref().unwrap_or(&"".to_string()).clone(),
                ),
            ])
            .start(&tracer);
        let cx = Context::current_with_span(span);

        let mut update_request = UpdateCategoryRequest {
            id: req.id.unwrap_or_default(),
            ..Default::default()
        };
        if let Some(name) = &req.name {
            update_request.name = name.to_string();
        }

        let mut request = Request::new(update_request);
        self.inject_trace_context(&cx, &mut request);

        let response = self.client.lock().await.update_category(request).await;

        self.add_completion_event(&cx, &response, "update_category_response".to_string());

        response
            .map(|resp| {
                let inner = resp.into_inner();
                ApiResponse {
                    status: inner.status,
                    message: inner.message,
                    data: inner.data.into(),
                }
            })
            .map_err(|status| ErrorResponse {
                status: status.code().to_string(),
                message: status.message().to_string(),
            })
    }

    async fn delete(&self, id: &i32) -> Result<ApiResponse<()>, ErrorResponse> {
        self.metrics.lock().await.inc_requests(Method::Delete);

        let tracer = self.get_tracer();
        let span = tracer
            .span_builder("DeleteCategory")
            .with_kind(SpanKind::Server)
            .with_attributes([
                KeyValue::new("component", "category"),
                KeyValue::new("category.id", id.to_string()),
            ])
            .start(&tracer);
        let cx = Context::current_with_span(span);

        let mut request = Request::new(FindCategoryRequest { id: *id });
        self.inject_trace_context(&cx, &mut request);

        let response = self.client.lock().await.delete_category(request).await;

        self.add_completion_event(&cx, &response, "delete_category_response".to_string());

        response
            .map(|resp| {
                let inner = resp.into_inner();
                ApiResponse {
                    status: inner.status,
                    message: inner.message,
                    data: (),
                }
            })
            .map_err(|status| ErrorResponse {
                status: status.code().to_string(),
                message: status.message().to_string(),
            })
    }
}
