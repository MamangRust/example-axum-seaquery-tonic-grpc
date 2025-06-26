use async_trait::async_trait;
use genproto::post::{
    CreatePostRequest, FindAllPostRequest, FindPostRequest, UpdatePostRequest,
    posts_service_client::PostsServiceClient,
};
use opentelemetry::{
    Context, KeyValue,
    global::{self, BoxedTracer},
    trace::{SpanKind, TraceContextExt, Tracer},
};
use shared::{
    domain::{
        ApiResponse, ApiResponsePagination, CreatePostRequest as DomainCreatePostRequest,
        ErrorResponse, FindAllPostRequest as DomainFindAllPostRequest, PostRelationResponse,
        PostResponse, UpdatePostRequest as DomainUpdatePostRequest,
    },
    utils::{MetadataInjector, Method, Metrics},
};
use std::sync::Arc;
use tokio::sync::Mutex;
use tonic::{Request, Status, transport::Channel};

use crate::abstract_trait::PostsServiceTrait;

#[derive(Debug)]
pub struct PostsService {
    client: Arc<Mutex<PostsServiceClient<Channel>>>,
    metrics: Arc<Mutex<Metrics>>,
}

impl PostsService {
    pub fn new(
        client: Arc<Mutex<PostsServiceClient<Channel>>>,
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
impl PostsServiceTrait for PostsService {
    async fn find_all(
        &self,
        req: &DomainFindAllPostRequest,
    ) -> Result<ApiResponsePagination<Vec<PostResponse>>, ErrorResponse> {
        self.metrics.lock().await.inc_requests(Method::Get);

        let tracer = self.get_tracer();
        let span = tracer
            .span_builder("FindAllPost")
            .with_kind(SpanKind::Server)
            .with_attributes([
                KeyValue::new("component", "post"),
                KeyValue::new("page", req.page.to_string()),
                KeyValue::new("page_size", req.page_size.to_string()),
                KeyValue::new("search", req.search.clone()),
            ])
            .start(&tracer);

        let cx = Context::current_with_span(span);

        let mut request = Request::new(FindAllPostRequest {
            page: req.page,
            page_size: req.page_size,
            search: req.search.clone(),
        });
        self.inject_trace_context(&cx, &mut request);

        let response = self.client.lock().await.find_all_posts(request).await;

        self.add_completion_event(&cx, &response, "find_all_post_response".to_string());

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

    async fn find_relation(
        &self,
        id: &i32,
    ) -> Result<ApiResponse<PostRelationResponse>, ErrorResponse> {
        self.metrics.lock().await.inc_requests(Method::Get);

        let tracer = self.get_tracer();
        let span = tracer
            .span_builder("FindPostRelation")
            .with_kind(SpanKind::Server)
            .with_attributes([
                KeyValue::new("component", "post"),
                KeyValue::new("post.id", id.to_string()),
            ])
            .start(&tracer);
        let cx = Context::current_with_span(span);

        let mut request = Request::new(FindPostRequest { post_id: *id });
        self.inject_trace_context(&cx, &mut request);

        let response = self.client.lock().await.find_post_relation(request).await;

        self.add_completion_event(&cx, &response, "find_post_response".to_string());

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

    async fn find_by_id(&self, id: &i32) -> Result<ApiResponse<PostResponse>, ErrorResponse> {
        self.metrics.lock().await.inc_requests(Method::Get);

        let tracer = self.get_tracer();
        let span = tracer
            .span_builder("FindPostById")
            .with_kind(SpanKind::Server)
            .with_attributes([
                KeyValue::new("component", "post"),
                KeyValue::new("post.id", id.to_string()),
            ])
            .start(&tracer);
        let cx = Context::current_with_span(span);

        let mut request = Request::new(FindPostRequest { post_id: *id });
        self.inject_trace_context(&cx, &mut request);

        let response = self.client.lock().await.find_post(request).await;

        self.add_completion_event(&cx, &response, "find_post_response".to_string());

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
        req: &DomainCreatePostRequest,
    ) -> Result<ApiResponse<PostResponse>, ErrorResponse> {
        self.metrics.lock().await.inc_requests(Method::Post);

        let tracer = self.get_tracer();
        let span = tracer
            .span_builder("CreatePost")
            .with_kind(SpanKind::Server)
            .with_attributes([
                KeyValue::new("component", "post"),
                KeyValue::new("post.title", req.title.clone()),
            ])
            .start(&tracer);
        let cx = Context::current_with_span(span);

        let mut request = Request::new(CreatePostRequest {
            title: req.title.clone(),
            body: req.body.clone(),
            file: req.file.clone(),
            category_id: req.category_id,
            user_id: req.user_id,
            user_name: req.user_name.clone(),
        });

        self.inject_trace_context(&cx, &mut request);

        let response = self.client.lock().await.create_post(request).await;

        self.add_completion_event(&cx, &response, "create_post_response".to_string());

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
        req: &DomainUpdatePostRequest,
    ) -> Result<ApiResponse<PostResponse>, ErrorResponse> {
        self.metrics.lock().await.inc_requests(Method::Put);

        let tracer = self.get_tracer();
        let span = tracer
            .span_builder("UpdatePost")
            .with_kind(SpanKind::Server)
            .with_attributes([
                KeyValue::new("component", "post"),
                KeyValue::new("post.title", req.title.clone()),
            ])
            .start(&tracer);
        let cx = Context::current_with_span(span);

        let mut request = Request::new(UpdatePostRequest {
            post_id: req.post_id.unwrap_or_default(),
            title: req.title.clone(),
            body: req.body.clone(),
            file: req.file.clone(),
            category_id: req.category_id,
            user_id: req.user_id,
            user_name: req.user_name.clone(),
        });

        self.inject_trace_context(&cx, &mut request);

        let response = self.client.lock().await.update_post(request).await;

        self.add_completion_event(&cx, &response, "update_post_response".to_string());

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
            .span_builder("DeletePost")
            .with_kind(SpanKind::Server)
            .with_attributes([
                KeyValue::new("component", "post"),
                KeyValue::new("post.id", *id as i64),
            ])
            .start(&tracer);
        let cx = Context::current_with_span(span);

        let mut request = Request::new(FindPostRequest { post_id: *id });
        self.inject_trace_context(&cx, &mut request);

        let response = self.client.lock().await.delete_post(request).await;

        self.add_completion_event(&cx, &response, "delete_post_response".to_string());

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
