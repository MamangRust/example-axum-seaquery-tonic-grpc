use crate::{
    abstract_trait::{CommentServiceTrait, DynCommentRepository},
    domain::{
        ApiResponse, CommentResponse, CreateCommentRequest, ErrorResponse, UpdateCommentRequest,
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
pub struct CommentService {
    repository: DynCommentRepository,
    metrics: Arc<Mutex<Metrics>>,
}

impl std::fmt::Debug for CommentService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CommentService")
            .field("repository", &"DynCommentRepository")
            .finish()
    }
}

impl CommentService {
    pub fn new(repository: DynCommentRepository, metrics: Arc<Mutex<Metrics>>) -> Self {
        Self {
            repository,
            metrics,
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
impl CommentServiceTrait for CommentService {
    async fn get_comments(&self) -> Result<ApiResponse<Vec<CommentResponse>>, ErrorResponse> {
        info!("Getting all comments");

        self.metrics.lock().await.inc_requests(Method::Get);

        let tracer = self.get_tracer();

        let span = tracer
            .span_builder("GetComments")
            .with_kind(SpanKind::Server)
            .with_attributes([KeyValue::new("component", "comment")])
            .start(&tracer);
        let cx = Context::current_with_span(span);

        let mut request = Request::new(());
        self.inject_trace_context(&cx, &mut request);

        let comments = self
            .repository
            .find_all()
            .await
            .map_err(ErrorResponse::from)?;

        let response = comments.into_iter().map(CommentResponse::from).collect();

        self.add_completion_event(&cx, &Ok(()), "Comments retrieved successfully".to_string());

        Ok(ApiResponse {
            status: "success".to_string(),
            message: "Comments retrieved successfully".to_string(),
            data: response,
        })
    }

    async fn get_comment(
        &self,
        id: i32,
    ) -> Result<Option<ApiResponse<CommentResponse>>, ErrorResponse> {
        self.metrics.lock().await.inc_requests(Method::Get);

        let tracer = self.get_tracer();

        let span = tracer
            .span_builder("GetComment")
            .with_kind(SpanKind::Server)
            .with_attributes([
                KeyValue::new("component", "comment"),
                KeyValue::new("id", id.to_string()),
            ])
            .start(&tracer);
        let cx = Context::current_with_span(span);

        let mut request = Request::new(id);
        self.inject_trace_context(&cx, &mut request);

        let comment = self
            .repository
            .find_by_id(id)
            .await
            .map_err(ErrorResponse::from)?;

        self.add_completion_event(&cx, &Ok(()), "Comment retrieved successfully".to_string());

        if let Some(comment) = comment {
            Ok(Some(ApiResponse {
                status: "success".to_string(),
                message: "Comment retrieved successfully".to_string(),
                data: CommentResponse::from(comment),
            }))
        } else {
            Err(ErrorResponse::from(AppError::NotFound(format!(
                "Comment with id {id} not found",
            ))))
        }
    }

    async fn create_comment(
        &self,
        input: &CreateCommentRequest,
    ) -> Result<ApiResponse<CommentResponse>, ErrorResponse> {
        self.metrics.lock().await.inc_requests(Method::Post);

        let tracer = self.get_tracer();

        let span = tracer
            .span_builder("CreateComment")
            .with_kind(SpanKind::Server)
            .with_attributes([
                KeyValue::new("component", "comment"),
                KeyValue::new("comment.name", input.user_name_comment.clone()),
            ])
            .start(&tracer);
        let cx = Context::current_with_span(span);

        let mut request = Request::new(input.clone());

        self.inject_trace_context(&cx, &mut request);

        let comment = self
            .repository
            .create(input)
            .await
            .map_err(ErrorResponse::from)?;

        self.add_completion_event(&cx, &Ok(()), "Comment created successfully".to_string());

        Ok(ApiResponse {
            status: "success".to_string(),
            message: "Comment created successfully".to_string(),
            data: CommentResponse::from(comment),
        })
    }

    async fn update_comment(
        &self,
        input: &UpdateCommentRequest,
    ) -> Result<Option<ApiResponse<CommentResponse>>, ErrorResponse> {
        self.metrics.lock().await.inc_requests(Method::Put);

        let tracer = self.get_tracer();

        let span = tracer
            .span_builder("UpdateComment")
            .with_kind(SpanKind::Server)
            .with_attributes([
                KeyValue::new("component", "comment"),
                KeyValue::new("comment.name", input.user_name_comment.clone()),
            ])
            .start(&tracer);
        let cx = Context::current_with_span(span);

        let mut request = Request::new(input.clone());

        self.inject_trace_context(&cx, &mut request);

        let comment = self
            .repository
            .update(input)
            .await
            .map_err(ErrorResponse::from)?;

        self.add_completion_event(&cx, &Ok(()), "Comment updated successfully".to_string());

        Ok(Some(ApiResponse {
            status: "success".to_string(),
            message: "Comment updated successfully".to_string(),
            data: CommentResponse::from(comment),
        }))
    }

    async fn delete_comment(&self, id: i32) -> Result<ApiResponse<()>, ErrorResponse> {
        self.metrics.lock().await.inc_requests(Method::Delete);

        let tracer = self.get_tracer();

        let span = tracer
            .span_builder("DeleteComment")
            .with_kind(SpanKind::Server)
            .with_attributes([
                KeyValue::new("component", "comment"),
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

        self.add_completion_event(&cx, &Ok(()), "Comment deleted successfully".to_string());

        Ok(ApiResponse {
            status: "success".to_string(),
            message: "Comment deleted successfully".to_string(),
            data: (),
        })
    }
}
