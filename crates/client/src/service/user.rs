use async_trait::async_trait;
use genproto::user::{
    CreateUserRequest, DeleteUserRequest, FindAllUserRequest, FindUserByIdRequest,
    UpdateUserRequest, user_service_client::UserServiceClient,
};
use opentelemetry::{
    Context, KeyValue,
    global::{self, BoxedTracer},
    trace::{SpanKind, TraceContextExt, Tracer},
};
use shared::{
    domain::{
        ApiResponse, ApiResponsePagination, CreateUserRequest as DomainCreateUserRequest,
        ErrorResponse, FindAllUserRequest as DomainFindAllUserRequest,
        UpdateUserRequest as DomainUpdateUserRequest, UserResponse,
    },
    utils::{MetadataInjector, Method, Metrics},
};
use std::sync::Arc;
use tokio::sync::Mutex;
use tonic::{Request, Status, transport::Channel};

use crate::abstract_trait::UserServiceTrait;

#[derive(Debug)]
pub struct UserService {
    client: Arc<Mutex<UserServiceClient<Channel>>>,
    metrics: Arc<Mutex<Metrics>>,
}

impl UserService {
    pub fn new(
        client: Arc<Mutex<UserServiceClient<Channel>>>,
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
impl UserServiceTrait for UserService {
    async fn find_all(
        &self,
        req: &DomainFindAllUserRequest,
    ) -> Result<ApiResponsePagination<Vec<UserResponse>>, ErrorResponse> {
        self.metrics.lock().await.inc_requests(Method::Get);

        let tracer = self.get_tracer();
        let span = tracer
            .span_builder("GetUsers")
            .with_kind(SpanKind::Server)
            .with_attributes([
                KeyValue::new("component", "user"),
                KeyValue::new("page", req.page.to_string()),
                KeyValue::new("page_size", req.page_size.to_string()),
                KeyValue::new("search", req.search.clone()),
            ])
            .start(&tracer);

        let cx = Context::current_with_span(span);

        let mut request = Request::new(FindAllUserRequest {
            page: req.page,
            page_size: req.page_size,
            search: req.search.clone(),
        });
        self.inject_trace_context(&cx, &mut request);

        let response = self.client.lock().await.find_all_users(request).await;

        self.add_completion_event(&cx, &response, "find_all_user_response".to_string());

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

    async fn find_by_id(&self, id: &i32) -> Result<ApiResponse<UserResponse>, ErrorResponse> {
        self.metrics.lock().await.inc_requests(Method::Get);

        let tracer = self.get_tracer();
        let span = tracer
            .span_builder("FindUserById")
            .with_kind(SpanKind::Server)
            .with_attributes([
                KeyValue::new("component", "user"),
                KeyValue::new("user.id", id.to_string()),
            ])
            .start(&tracer);
        let cx = Context::current_with_span(span);

        let mut request = Request::new(FindUserByIdRequest { id: *id });
        self.inject_trace_context(&cx, &mut request);

        let response = self.client.lock().await.find_by_id(request).await;

        self.add_completion_event(&cx, &response, "find_user_by_id_response".to_string());

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
        req: &DomainCreateUserRequest,
    ) -> Result<ApiResponse<UserResponse>, ErrorResponse> {
        self.metrics.lock().await.inc_requests(Method::Post);

        let tracer = self.get_tracer();
        let span = tracer
            .span_builder("CreateUser")
            .with_kind(SpanKind::Server)
            .with_attributes([
                KeyValue::new("component", "user"),
                KeyValue::new("user.email", req.email.clone()),
            ])
            .start(&tracer);
        let cx = Context::current_with_span(span);

        let mut request = Request::new(CreateUserRequest {
            firstname: req.firstname.clone(),
            lastname: req.lastname.clone(),
            email: req.email.clone(),
            password: req.password.clone(),
        });
        self.inject_trace_context(&cx, &mut request);

        let response = self.client.lock().await.create_user(request).await;

        self.add_completion_event(&cx, &response, "create_user_response".to_string());

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
        req: &DomainUpdateUserRequest,
    ) -> Result<ApiResponse<UserResponse>, ErrorResponse> {
        self.metrics.lock().await.inc_requests(Method::Put);

        let tracer = self.get_tracer();
        let span = tracer
            .span_builder("UpdateUser")
            .with_kind(SpanKind::Server)
            .with_attributes([
                KeyValue::new("component", "user"),
                KeyValue::new(
                    "user.email",
                    req.email.as_ref().unwrap_or(&"".to_string()).clone(),
                ),
            ])
            .start(&tracer);
        let cx = Context::current_with_span(span);

        let mut update_request = UpdateUserRequest {
            id: req.id.unwrap_or_default(),
            ..Default::default()
        };
        if let Some(firstname) = &req.firstname {
            update_request.firstname = firstname.to_string();
        }
        if let Some(lastname) = &req.lastname {
            update_request.lastname = lastname.to_string();
        }
        if let Some(email) = &req.email {
            update_request.email = email.to_string();
        }
        if let Some(password) = &req.password {
            update_request.password = password.to_string();
        }

        let mut request = Request::new(update_request);
        self.inject_trace_context(&cx, &mut request);

        let response = self.client.lock().await.update_user(request).await;

        self.add_completion_event(&cx, &response, "update_user_response".to_string());

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

    async fn delete(&self, email: &str) -> Result<ApiResponse<()>, ErrorResponse> {
        self.metrics.lock().await.inc_requests(Method::Delete);

        let tracer = self.get_tracer();
        let span = tracer
            .span_builder("DeleteUser")
            .with_kind(SpanKind::Server)
            .with_attributes([
                KeyValue::new("component", "user"),
                KeyValue::new("user.email", email.to_string()),
            ])
            .start(&tracer);
        let cx = Context::current_with_span(span);

        let mut request = Request::new(DeleteUserRequest {
            email: email.to_string(),
        });
        self.inject_trace_context(&cx, &mut request);

        let response = self.client.lock().await.delete_user(request).await;

        self.add_completion_event(&cx, &response, "delete_user_response".to_string());

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
