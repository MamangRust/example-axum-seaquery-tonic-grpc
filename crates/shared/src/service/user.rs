use crate::{
    abstract_trait::{DynUserRepository, UserServiceTrait},
    cache::CacheStore,
    domain::{
        ApiResponse, ApiResponsePagination, CreateUserRequest, ErrorResponse, FindAllUserRequest,
        Pagination, UpdateUserRequest, UserResponse,
    },
    utils::{AppError, MetadataInjector, Method, Metrics, Status as StatusUtils, TracingContext},
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
pub struct UserService {
    repository: DynUserRepository,
    metrics: Arc<Mutex<Metrics>>,
    cache_store: Arc<CacheStore>,
}

impl UserService {
    pub async fn new(
        repository: DynUserRepository,
        metrics: Arc<Mutex<Metrics>>,
        registry: &mut Registry,
        cache_store: Arc<CacheStore>,
    ) -> Self {
        registry.register(
            "user_service_request_counter",
            "Total number of requests to the UserService",
            metrics.lock().await.request_counter.clone(),
        );
        registry.register(
            "user_service_request_duration",
            "Histogram of request durations for the UserService",
            metrics.lock().await.request_duration.clone(),
        );

        Self {
            repository,
            metrics,
            cache_store,
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
impl UserServiceTrait for UserService {
    async fn get_users(
        &self,
        req: FindAllUserRequest,
    ) -> Result<ApiResponsePagination<Vec<UserResponse>>, ErrorResponse> {
        let method = Method::Get;
        let page = req.page.max(1);
        let page_size = req.page_size.max(1);
        let search = if req.search.is_empty() {
            None
        } else {
            Some(req.search.clone())
        };

        let tracing_ctx = self.start_tracing(
            "GetUsers",
            vec![
                KeyValue::new("component", "user"),
                KeyValue::new("page", page.to_string()),
                KeyValue::new("page_size", page_size.to_string()),
                KeyValue::new("search", search.clone().unwrap_or_default()),
            ],
        );

        let mut request = Request::new(FindAllUserRequest {
            page,
            page_size,
            search: search.clone().unwrap_or_default(),
        });
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let cache_key = format!(
            "users:page={page}:size={page_size}:search={}",
            search.clone().unwrap_or_default()
        );

        if let Some(cached) = self
            .cache_store
            .get_from_cache::<ApiResponsePagination<Vec<UserResponse>>>(&cache_key)
            .await
        {
            self.complete_tracing_success(&tracing_ctx, method, "Users retrieved from cache")
                .await;

            return Ok(cached);
        }

        match self.repository.find_all(page, page_size, search).await {
            Ok((users, total_items)) => {
                info!("Found {} users", users.len());
                let total_pages = (total_items as f64 / page_size as f64).ceil() as i32;
                let user_responses = users.into_iter().map(UserResponse::from).collect();

                let response = ApiResponsePagination {
                    status: "success".to_string(),
                    message: "Users retrieved successfully".to_string(),
                    data: user_responses,
                    pagination: Pagination {
                        page,
                        page_size,
                        total_items,
                        total_pages,
                    },
                };

                self.cache_store
                    .set_to_cache(&cache_key, &response, Duration::from_secs(60 * 5))
                    .await;

                self.complete_tracing_success(&tracing_ctx, method, "Users retrieved successfully")
                    .await;

                Ok(response)
            }
            Err(err) => {
                self.complete_tracing_error(
                    &tracing_ctx,
                    method,
                    &format!("Failed to retrieve users: {err}"),
                )
                .await;

                Err(ErrorResponse::from(err))
            }
        }
    }
    async fn create_user(
        &self,
        input: &CreateUserRequest,
    ) -> Result<ApiResponse<UserResponse>, ErrorResponse> {
        let method = Method::Post;
        let tracing_ctx = self.start_tracing(
            "CreateUser",
            vec![
                KeyValue::new("component", "user"),
                KeyValue::new("user.email", input.email.clone()),
            ],
        );

        let mut request = Request::new(input.clone());
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        match self.repository.find_by_email_exists(&input.email).await {
            Ok(true) => {
                self.complete_tracing_error(&tracing_ctx, Method::Post, "Email already exists")
                    .await;
                return Err(ErrorResponse::from(AppError::EmailAlreadyExists));
            }
            Ok(false) => (),
            Err(err) => {
                self.complete_tracing_error(
                    &tracing_ctx,
                    method,
                    &format!("Error checking email: {err}"),
                )
                .await;
                return Err(ErrorResponse::from(err));
            }
        }

        match self.repository.create_user(input).await {
            Ok(user) => {
                let response = ApiResponse {
                    status: "success".to_string(),
                    message: "User created successfully".to_string(),
                    data: UserResponse::from(user),
                };

                self.complete_tracing_success(&tracing_ctx, method, "User created successfully")
                    .await;

                Ok(response)
            }
            Err(err) => {
                self.complete_tracing_error(
                    &tracing_ctx,
                    method,
                    &format!("Failed to create user: {err}"),
                )
                .await;

                Err(ErrorResponse::from(err))
            }
        }
    }

    async fn find_by_id(
        &self,
        id: i32,
    ) -> Result<Option<ApiResponse<UserResponse>>, ErrorResponse> {
        let method = Method::Get;
        let tracing_ctx = self.start_tracing(
            "FindUserById",
            vec![
                KeyValue::new("component", "user"),
                KeyValue::new("user.id", id.to_string()),
            ],
        );

        let mut request = Request::new(id);
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let cache_key = format!("user:id={id}");

        if let Some(cached) = self
            .cache_store
            .get_from_cache::<ApiResponse<UserResponse>>(&cache_key)
            .await
        {
            self.complete_tracing_success(&tracing_ctx, method, "User retrieved from cache")
                .await;

            return Ok(Some(cached));
        }

        match self.repository.find_by_id(id).await {
            Ok(Some(user)) => {
                let response = Some(ApiResponse {
                    status: "success".to_string(),
                    message: "User retrieved successfully".to_string(),
                    data: UserResponse::from(user),
                });

                self.cache_store
                    .set_to_cache(&cache_key, &response, Duration::from_secs(60 * 5))
                    .await;

                self.complete_tracing_success(&tracing_ctx, method, "User retrieved successfully")
                    .await;

                Ok(response)
            }
            Ok(None) => {
                self.complete_tracing_error(
                    &tracing_ctx,
                    method,
                    &format!("User with id {id} not found"),
                )
                .await;

                Err(ErrorResponse::from(AppError::NotFound(format!(
                    "User with id {id} not found"
                ))))
            }
            Err(err) => {
                self.complete_tracing_error(
                    &tracing_ctx,
                    method,
                    &format!("Error retrieving user: {err}"),
                )
                .await;

                Err(ErrorResponse::from(err))
            }
        }
    }

    async fn update_user(
        &self,
        input: &UpdateUserRequest,
    ) -> Result<Option<ApiResponse<UserResponse>>, ErrorResponse> {
        let method = Method::Put;
        let tracing_ctx = self.start_tracing(
            "UpdateUser",
            vec![
                KeyValue::new("component", "user"),
                KeyValue::new("user.email", input.email.clone().unwrap_or_default()),
            ],
        );

        let mut request = Request::new(input.clone());
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        match self.repository.update_user(input).await {
            Ok(user) => {
                let user_email = user.email.clone();

                let response = Some(ApiResponse {
                    status: "success".to_string(),
                    message: "User updated successfully".to_string(),
                    data: UserResponse::from(user),
                });

                self.cache_store
                    .set_to_cache(
                        &format!("user:email={user_email}"),
                        &response,
                        Duration::from_secs(60 * 5),
                    )
                    .await;

                self.complete_tracing_success(&tracing_ctx, method, "User updated successfully")
                    .await;

                Ok(response)
            }
            Err(err) => {
                self.complete_tracing_error(
                    &tracing_ctx,
                    method,
                    &format!("Failed to update user: {err}"),
                )
                .await;

                Err(ErrorResponse::from(err))
            }
        }
    }

    async fn delete_user(&self, email: &str) -> Result<ApiResponse<()>, ErrorResponse> {
        let method = Method::Delete;
        let tracing_ctx = self.start_tracing(
            "DeleteUser",
            vec![
                KeyValue::new("component", "user"),
                KeyValue::new("user.email", email.to_string()),
            ],
        );

        let mut request = Request::new(email.to_string());
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        match self.repository.delete_user(email).await {
            Ok(_) => {
                let response = ApiResponse {
                    status: "success".to_string(),
                    message: "User deleted successfully".to_string(),
                    data: (),
                };

                self.cache_store
                    .delete_from_cache(&format!("user:email={email}"))
                    .await;

                self.complete_tracing_success(&tracing_ctx, method, "User deleted successfully")
                    .await;

                Ok(response)
            }
            Err(err) => {
                self.complete_tracing_error(
                    &tracing_ctx,
                    method,
                    &format!("Failed to delete user: {err}"),
                )
                .await;

                Err(ErrorResponse::from(err))
            }
        }
    }
}
