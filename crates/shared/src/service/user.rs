use crate::{
    abstract_trait::{DynUserRepository, UserServiceTrait},
    domain::{
        ApiResponse, ApiResponsePagination, CreateUserRequest, ErrorResponse, FindAllUserRequest,
        Pagination, UpdateUserRequest, UserResponse,
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
pub struct UserService {
    repository: DynUserRepository,
    metrics: Arc<Mutex<Metrics>>,
}

impl UserService {
    pub fn new(repository: DynUserRepository, metrics: Arc<Mutex<Metrics>>) -> Self {
        Self {
            repository,
            metrics,
        }
    }

    fn get_tracer(&self) -> BoxedTracer {
        global::tracer("user-service")
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
impl UserServiceTrait for UserService {
    async fn get_users(
        &self,
        req: FindAllUserRequest,
    ) -> Result<ApiResponsePagination<Vec<UserResponse>>, ErrorResponse> {
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
            .span_builder("GetUsers")
            .with_kind(SpanKind::Server)
            .with_attributes([
                KeyValue::new("component", "user"),
                KeyValue::new("page", page.to_string()),
                KeyValue::new("page_size", page_size.to_string()),
                KeyValue::new("search", search.clone().unwrap_or("".to_string())),
            ])
            .start(&tracer);
        let cx = Context::current_with_span(span);

        let mut request = Request::new(FindAllUserRequest {
            page,
            page_size,
            search: search.clone().unwrap_or("".to_string()),
        });

        self.inject_trace_context(&cx, &mut request);

        let (users, total_items) = self
            .repository
            .find_all(page, page_size, search)
            .await
            .map_err(ErrorResponse::from)?;

        info!("Found {} users", users.len());

        let total_pages = (total_items as f64 / page_size as f64).ceil() as i32;

        let user_responses: Vec<UserResponse> = users.into_iter().map(UserResponse::from).collect();

        self.add_completion_event(&cx, &Ok(()), "Users retrieved successfully".to_string());

        Ok(ApiResponsePagination {
            status: "success".to_string(),
            message: "Users retrieved successfully".to_string(),
            data: user_responses,
            pagination: Pagination {
                page,
                page_size,
                total_items,
                total_pages,
            },
        })
    }
    async fn create_user(
        &self,
        input: &CreateUserRequest,
    ) -> Result<ApiResponse<UserResponse>, ErrorResponse> {
        self.metrics.lock().await.inc_requests(Method::Post);

        let tracer = self.get_tracer();

        let span = tracer
            .span_builder("CreateUser")
            .with_kind(SpanKind::Client)
            .with_attributes([
                KeyValue::new("component", "user"),
                KeyValue::new("user.email", input.email.clone()),
            ])
            .start(&tracer);
        let cx = Context::current_with_span(span);

        let mut request = Request::new(input.clone());
        self.inject_trace_context(&cx, &mut request);

        let exists = self
            .repository
            .find_by_email_exists(&input.email)
            .await
            .map_err(ErrorResponse::from)?;

        if exists {
            return Err(ErrorResponse::from(AppError::EmailAlreadyExists));
        }

        let user = self
            .repository
            .create_user(input)
            .await
            .map_err(ErrorResponse::from)?;

        self.add_completion_event(&cx, &Ok(()), "User created successfully".to_string());

        Ok(ApiResponse {
            status: "success".to_string(),
            message: "User created successfully".to_string(),
            data: UserResponse::from(user),
        })
    }

    async fn find_by_id(
        &self,
        id: i32,
    ) -> Result<Option<ApiResponse<UserResponse>>, ErrorResponse> {
        self.metrics.lock().await.inc_requests(Method::Get);

        let tracer = self.get_tracer();

        let span = tracer
            .span_builder("FindUserById")
            .with_kind(SpanKind::Client)
            .with_attributes([
                KeyValue::new("component", "user"),
                KeyValue::new("user.id", id.to_string()),
            ])
            .start(&tracer);
        let cx = Context::current_with_span(span);

        let mut request = Request::new(id);
        self.inject_trace_context(&cx, &mut request);

        let user = self
            .repository
            .find_by_id(id)
            .await
            .map_err(ErrorResponse::from)?;

        self.add_completion_event(&cx, &Ok(()), "User retrieved successfully".to_string());

        if let Some(user) = user {
            Ok(Some(ApiResponse {
                status: "success".to_string(),
                message: "User retrieved successfully".to_string(),
                data: UserResponse::from(user),
            }))
        } else {
            Err(ErrorResponse::from(AppError::NotFound(format!(
                "User with id {id} not found",
            ))))
        }
    }

    async fn update_user(
        &self,
        input: &UpdateUserRequest,
    ) -> Result<Option<ApiResponse<UserResponse>>, ErrorResponse> {
        self.metrics.lock().await.inc_requests(Method::Put);

        let tracer = self.get_tracer();

        let span = tracer
            .span_builder("UpdateUser")
            .with_kind(SpanKind::Server)
            .with_attributes([
                KeyValue::new("component", "user"),
                KeyValue::new("user.email", input.email.clone().unwrap_or("".to_string())),
            ])
            .start(&tracer);
        let cx = Context::current_with_span(span);

        let mut request = Request::new(input.clone());
        self.inject_trace_context(&cx, &mut request);

        let user = self
            .repository
            .update_user(input)
            .await
            .map_err(ErrorResponse::from)?;

        self.add_completion_event(&cx, &Ok(()), "User updated successfully".to_string());

        Ok(Some(ApiResponse {
            status: "success".to_string(),
            message: "User updated successfully".to_string(),
            data: UserResponse::from(user),
        }))
    }

    async fn delete_user(&self, email: &str) -> Result<ApiResponse<()>, ErrorResponse> {
        self.metrics.lock().await.inc_requests(Method::Delete);

        let tracer = self.get_tracer();

        let email_str = email.to_string();

        let span = tracer
            .span_builder("DeleteUser")
            .with_kind(SpanKind::Server)
            .with_attributes([
                KeyValue::new("component", "user"),
                KeyValue::new("user.email", email_str.clone()),
            ])
            .start(&tracer);
        let cx = Context::current_with_span(span);

        let mut request = Request::new(email_str.clone());

        self.inject_trace_context(&cx, &mut request);

        self.repository
            .delete_user(email)
            .await
            .map_err(ErrorResponse::from)?;

        self.add_completion_event(&cx, &Ok(()), "User deleted successfully".to_string());

        Ok(ApiResponse {
            status: "success".to_string(),
            message: "User deleted successfully".to_string(),
            data: (),
        })
    }
}
