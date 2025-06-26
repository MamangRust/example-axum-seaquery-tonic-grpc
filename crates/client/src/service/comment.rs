use async_trait::async_trait;
use genproto::comment::{
    CreateCommentRequest, Empty, FindCommentRequest, UpdateCommentRequest,
    comment_service_client::CommentServiceClient,
};
use opentelemetry::{
    Context, KeyValue,
    global::{self, BoxedTracer},
    trace::{SpanKind, TraceContextExt, Tracer},
};
use shared::{
    domain::{
        ApiResponse, CommentResponse, CreateCommentRequest as DomainCreateCommentRequest,
        ErrorResponse, UpdateCommentRequest as DomainUpdateCommentRequest,
    },
    utils::{MetadataInjector, Method, Metrics},
};
use std::sync::Arc;
use tokio::sync::Mutex;
use tonic::{Request, Status, transport::Channel};

use crate::abstract_trait::CommentServiceTrait;

#[derive(Debug)]
pub struct CommentService {
    client: Arc<Mutex<CommentServiceClient<Channel>>>,
    metrics: Arc<Mutex<Metrics>>,
}

impl CommentService {
    pub fn new(
        client: Arc<Mutex<CommentServiceClient<Channel>>>,
        metrics: Arc<Mutex<Metrics>>,
    ) -> Self {
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
impl CommentServiceTrait for CommentService {
    async fn find_all(&self) -> Result<ApiResponse<Vec<CommentResponse>>, ErrorResponse> {
        self.metrics.lock().await.inc_requests(Method::Get);

        let tracer = self.get_tracer();
        let span = tracer
            .span_builder("FindAllComments")
            .with_kind(SpanKind::Client)
            .start(&tracer);
        let cx = Context::current_with_span(span);

        let mut request = Request::new(Empty {});
        self.inject_trace_context(&cx, &mut request);

        let response = self.client.lock().await.get_comments(request).await;

        self.add_completion_event(&cx, &response, "find_all_comments_response".to_string());

        response
            .map(|resp| {
                let inner = resp.into_inner();
                ApiResponse {
                    status: inner.status,
                    message: inner.message,
                    data: inner.data.into_iter().map(|c| c.into()).collect(),
                }
            })
            .map_err(|status| ErrorResponse {
                status: status.code().to_string(),
                message: status.message().to_string(),
            })
    }

    async fn find_by_id(&self, id: &i32) -> Result<ApiResponse<CommentResponse>, ErrorResponse> {
        self.metrics.lock().await.inc_requests(Method::Get);

        let tracer = self.get_tracer();
        let span = tracer
            .span_builder("FindCommentById")
            .with_kind(SpanKind::Client)
            .with_attributes([KeyValue::new("comment.id", *id as i64)])
            .start(&tracer);
        let cx = Context::current_with_span(span);

        let mut request = Request::new(FindCommentRequest { id: *id });
        self.inject_trace_context(&cx, &mut request);

        let response = self.client.lock().await.get_comment(request).await;

        self.add_completion_event(&cx, &response, "find_comment_by_id_response".to_string());

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
        req: &DomainCreateCommentRequest,
    ) -> Result<ApiResponse<CommentResponse>, ErrorResponse> {
        self.metrics.lock().await.inc_requests(Method::Post);

        let tracer = self.get_tracer();
        let span = tracer
            .span_builder("CreateComment")
            .with_kind(SpanKind::Client)
            .with_attributes([
                KeyValue::new("comment.post_id", req.id_post_comment as i64),
                KeyValue::new("comment.user", req.user_name_comment.clone()),
            ])
            .start(&tracer);
        let cx = Context::current_with_span(span);

        let mut request = Request::new(CreateCommentRequest {
            id_post_comment: req.id_post_comment,
            user_name_comment: req.user_name_comment.clone(),
            comment: req.comment.clone(),
        });
        self.inject_trace_context(&cx, &mut request);

        let response = self.client.lock().await.create_comment(request).await;

        self.add_completion_event(&cx, &response, "create_comment_response".to_string());

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
        req: &DomainUpdateCommentRequest,
    ) -> Result<ApiResponse<CommentResponse>, ErrorResponse> {
        self.metrics.lock().await.inc_requests(Method::Put);

        let tracer = self.get_tracer();
        let span = tracer
            .span_builder("UpdateComment")
            .with_kind(SpanKind::Client)
            .with_attributes([KeyValue::new("comment.user", req.user_name_comment.clone())])
            .start(&tracer);
        let cx = Context::current_with_span(span);

        let mut request = Request::new(UpdateCommentRequest {
            id_post_comment: req.id_post_comment.unwrap_or_default(),
            user_name_comment: req.user_name_comment.clone(),
            comment: req.comment.clone(),
        });
        self.inject_trace_context(&cx, &mut request);

        let response = self.client.lock().await.update_comment(request).await;

        self.add_completion_event(&cx, &response, "update_comment_response".to_string());

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
            .span_builder("DeleteComment")
            .with_kind(SpanKind::Client)
            .with_attributes([KeyValue::new("comment.id", *id as i64)])
            .start(&tracer);
        let cx = Context::current_with_span(span);

        let mut request = Request::new(FindCommentRequest { id: *id });
        self.inject_trace_context(&cx, &mut request);

        let response = self.client.lock().await.delete_comment(request).await;

        self.add_completion_event(&cx, &response, "delete_comment_response".to_string());

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
