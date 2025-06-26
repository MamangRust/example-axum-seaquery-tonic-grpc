use crate::{
    abstract_trait::{DynPostsRepository, PostsServiceTrait},
    domain::{
        ApiResponse, ApiResponsePagination, CreatePostRequest, ErrorResponse, FindAllPostRequest,
        Pagination, PostRelationResponse, PostResponse, UpdatePostRequest,
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

#[derive(Clone)]
pub struct PostService {
    repository: DynPostsRepository,
    metrics: Arc<Mutex<Metrics>>,
}

impl std::fmt::Debug for PostService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PostService").finish()
    }
}

impl PostService {
    pub fn new(repository: DynPostsRepository, metrics: Arc<Mutex<Metrics>>) -> Self {
        Self {
            repository,
            metrics,
        }
    }

    fn get_tracer(&self) -> BoxedTracer {
        global::tracer("post-service")
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
impl PostsServiceTrait for PostService {
    async fn get_all_posts(
        &self,
        req: FindAllPostRequest,
    ) -> Result<ApiResponsePagination<Vec<PostResponse>>, ErrorResponse> {
        self.metrics.lock().await.inc_requests(Method::Get);

        let tracer = self.get_tracer();

        let page = req.page.max(1);
        let page_size = req.page_size.max(1);
        let search = if req.search.is_empty() {
            None
        } else {
            Some(req.search.clone())
        };

        let span = tracer
            .span_builder("GetAllPosts")
            .with_kind(SpanKind::Server)
            .with_attributes([
                KeyValue::new("component", "post"),
                KeyValue::new("page", page.to_string()),
                KeyValue::new("page_size", page_size.to_string()),
                KeyValue::new("search", search.clone().unwrap_or("".to_string())),
            ])
            .start(&tracer);
        let cx = Context::current_with_span(span);

        let mut request = Request::new(FindAllPostRequest {
            page,
            page_size,
            search: search.clone().unwrap_or("".to_string()),
        });

        self.inject_trace_context(&cx, &mut request);

        let (posts, total_items) = self
            .repository
            .get_all_posts(page, page_size, search)
            .await
            .map_err(ErrorResponse::from)?;

        let responses: Vec<PostResponse> = posts.into_iter().map(PostResponse::from).collect();

        let total_pages = (total_items as f64 / req.page_size as f64).ceil() as i32;

        self.add_completion_event(&cx, &Ok(()), "Posts retrieved successfully".to_string());

        Ok(ApiResponsePagination {
            status: "success".to_string(),
            message: "Posts retrieved successfully".to_string(),
            data: responses,
            pagination: Pagination {
                page: req.page,
                page_size: req.page_size,
                total_items,
                total_pages,
            },
        })
    }

    async fn get_post(
        &self,
        post_id: i32,
    ) -> Result<Option<ApiResponse<PostResponse>>, ErrorResponse> {
        self.metrics.lock().await.inc_requests(Method::Get);

        let tracer = self.get_tracer();

        let span = tracer
            .span_builder("GetPost")
            .with_kind(SpanKind::Server)
            .with_attributes([
                KeyValue::new("component", "post"),
                KeyValue::new("id", post_id.to_string()),
            ])
            .start(&tracer);
        let cx = Context::current_with_span(span);

        let post = self
            .repository
            .get_post(post_id)
            .await
            .map_err(ErrorResponse::from)?;

        self.add_completion_event(&cx, &Ok(()), "Post retrieved successfully".to_string());

        if let Some(post) = post {
            Ok(Some(ApiResponse {
                status: "success".to_string(),
                message: "Post retrieved successfully".to_string(),
                data: PostResponse::from(post),
            }))
        } else {
            Err(ErrorResponse::from(AppError::NotFound(format!(
                "Posts with id {} not found",
                post_id
            ))))
        }
    }

    async fn get_post_relation(
        &self,
        post_id: i32,
    ) -> Result<ApiResponse<PostRelationResponse>, ErrorResponse> {
        self.metrics.lock().await.inc_requests(Method::Get);

        let tracer = self.get_tracer();

        let span = tracer
            .span_builder("GetPostRelation")
            .with_kind(SpanKind::Server)
            .with_attributes([
                KeyValue::new("component", "post"),
                KeyValue::new("id", post_id.to_string()),
            ])
            .start(&tracer);
        let cx = Context::current_with_span(span);

        let mut request = Request::new(post_id);
        self.inject_trace_context(&cx, &mut request);

        let relations = self
            .repository
            .get_post_relation(post_id)
            .await
            .map_err(ErrorResponse::from)?;

        let first_relation = relations
            .into_iter()
            .next()
            .ok_or_else(|| AppError::NotFound("Post relation not found".to_string()))?;

        self.add_completion_event(
            &cx,
            &Ok(()),
            "Post relation retrieved successfully".to_string(),
        );

        Ok(ApiResponse {
            status: "success".to_string(),
            message: "Post relation retrieved successfully".to_string(),
            data: first_relation,
        })
    }

    async fn create_post(
        &self,
        input: &CreatePostRequest,
    ) -> Result<ApiResponse<PostResponse>, ErrorResponse> {
        self.metrics.lock().await.inc_requests(Method::Post);

        let tracer = self.get_tracer();

        let span = tracer
            .span_builder("CreatePost")
            .with_kind(SpanKind::Server)
            .with_attributes([
                KeyValue::new("component", "post"),
                KeyValue::new("title", input.title.clone()),
            ])
            .start(&tracer);
        let cx = Context::current_with_span(span);

        let mut request = Request::new(input.clone());

        self.inject_trace_context(&cx, &mut request);

        let post = self
            .repository
            .create_post(input)
            .await
            .map_err(ErrorResponse::from)?;

        self.add_completion_event(&cx, &Ok(()), "Post created successfully".to_string());

        Ok(ApiResponse {
            status: "success".to_string(),
            message: "Post created successfully".to_string(),
            data: PostResponse::from(post),
        })
    }

    async fn update_post(
        &self,
        input: &UpdatePostRequest,
    ) -> Result<ApiResponse<PostResponse>, ErrorResponse> {
        self.metrics.lock().await.inc_requests(Method::Put);

        let tracer = self.get_tracer();

        let span = tracer
            .span_builder("UpdatePost")
            .with_kind(SpanKind::Server)
            .with_attributes([
                KeyValue::new("component", "post"),
                KeyValue::new("title", input.title.clone()),
            ])
            .start(&tracer);
        let cx = Context::current_with_span(span);

        let mut request = Request::new(input.clone());

        self.inject_trace_context(&cx, &mut request);

        let post = self
            .repository
            .update_post(input)
            .await
            .map_err(ErrorResponse::from)?;

        self.add_completion_event(&cx, &Ok(()), "Post updated successfully".to_string());

        Ok(ApiResponse {
            status: "success".to_string(),
            message: "Post updated successfully".to_string(),
            data: PostResponse::from(post),
        })
    }

    async fn delete_post(&self, post_id: i32) -> Result<ApiResponse<()>, ErrorResponse> {
        self.metrics.lock().await.inc_requests(Method::Delete);

        let tracer = self.get_tracer();

        let span = tracer
            .span_builder("DeletePost")
            .with_kind(SpanKind::Server)
            .with_attributes([
                KeyValue::new("component", "post"),
                KeyValue::new("id", post_id.to_string()),
            ])
            .start(&tracer);
        let cx = Context::current_with_span(span);

        let mut request = Request::new(post_id);
        self.inject_trace_context(&cx, &mut request);

        self.repository
            .delete_post(post_id)
            .await
            .map_err(ErrorResponse::from)?;

        self.add_completion_event(&cx, &Ok(()), "Post deleted successfully".to_string());

        Ok(ApiResponse {
            status: "success".to_string(),
            message: "Post deleted successfully".to_string(),
            data: (),
        })
    }
}
