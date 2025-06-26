use async_trait::async_trait;
use genproto::auth::{
    GetMeRequest, LoginRequest, RegisterRequest, auth_service_client::AuthServiceClient,
};
use opentelemetry::{
    Context, KeyValue,
    global::{self, BoxedTracer},
    trace::{SpanKind, TraceContextExt, Tracer},
};
use shared::{
    domain::{
        ApiResponse, ErrorResponse, LoginRequest as LoginDomainRequest,
        RegisterRequest as RegisterDomainRequest, UserResponse,
    },
    utils::{MetadataInjector, Method, Metrics},
};
use std::sync::Arc;
use tokio::sync::Mutex;
use tonic::{Request, Status, transport::Channel};

use crate::abstract_trait::AuthServiceTrait;

#[derive(Debug)]
pub struct AuthService {
    client: Arc<Mutex<AuthServiceClient<Channel>>>,
    metrics: Arc<Mutex<Metrics>>,
}

impl AuthService {
    pub fn new(
        client: Arc<Mutex<AuthServiceClient<Channel>>>,
        metrics: Arc<Mutex<Metrics>>,
    ) -> Self {
        Self { client, metrics }
    }

    fn get_tracer(&self) -> BoxedTracer {
        global::tracer("auth-service-client")
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
impl AuthServiceTrait for AuthService {
    async fn register(
        &self,
        request_data: RegisterDomainRequest,
    ) -> Result<ApiResponse<UserResponse>, ErrorResponse> {
        self.metrics.lock().await.inc_requests(Method::Post);

        let tracer = self.get_tracer();
        let span = tracer
            .span_builder("RegisterUser")
            .with_kind(SpanKind::Client)
            .with_attributes([
                KeyValue::new("component", "auth"),
                KeyValue::new("user.email", request_data.email.clone()),
            ])
            .start(&tracer);
        let cx = Context::current_with_span(span);

        let mut request = Request::new(RegisterRequest {
            firstname: request_data.firstname.clone(),
            lastname: request_data.lastname.clone(),
            email: request_data.email.clone(),
            password: request_data.password.clone(),
        });

        self.inject_trace_context(&cx, &mut request);

        let result = {
            let mut client = self.client.lock().await;
            client.register_user(request).await
        };

        self.add_completion_event(&cx, &result, "register_user_response".to_string());

        result
            .map_err(|status| ErrorResponse {
                status: status.code().to_string(),
                message: status.message().to_string(),
            })
            .map(|resp| {
                let inner = resp.into_inner();
                ApiResponse {
                    status: inner.status,
                    message: inner.message,
                    data: inner.data.into(),
                }
            })
    }

    async fn login(
        &self,
        request_data: LoginDomainRequest,
    ) -> Result<ApiResponse<String>, ErrorResponse> {
        self.metrics.lock().await.inc_requests(Method::Post);

        let tracer = self.get_tracer();
        let span = tracer
            .span_builder("LoginUser")
            .with_kind(SpanKind::Client)
            .with_attributes([
                KeyValue::new("component", "auth"),
                KeyValue::new("user.email", request_data.email.clone()),
            ])
            .start(&tracer);
        let cx = Context::current_with_span(span);

        let mut request = Request::new(LoginRequest {
            email: request_data.email.clone(),
            password: request_data.password.clone(),
        });

        self.inject_trace_context(&cx, &mut request);

        let result = {
            let mut client = self.client.lock().await;
            client.login_user(request).await
        };

        self.add_completion_event(&cx, &result, "login_user_response".to_string());

        result
            .map(|resp| {
                let inner = resp.into_inner();
                ApiResponse {
                    status: inner.status,
                    message: inner.message,
                    data: inner.data,
                }
            })
            .map_err(|status| ErrorResponse {
                status: status.code().to_string(),
                message: status.message().to_string(),
            })
    }

    async fn get_me(&self, id: i32) -> Result<ApiResponse<UserResponse>, ErrorResponse> {
        self.metrics.lock().await.inc_requests(Method::Get);

        let tracer = self.get_tracer();
        let span = tracer
            .span_builder("GetMe")
            .with_kind(SpanKind::Client)
            .with_attributes([
                KeyValue::new("component", "user"),
                KeyValue::new("user.id", id.to_string()),
            ])
            .start(&tracer);
        let cx = Context::current_with_span(span);

        let mut request = Request::new(GetMeRequest { id });
        self.inject_trace_context(&cx, &mut request);

        let result = {
            let mut client = self.client.lock().await;
            client.get_me(request).await
        };

        self.add_completion_event(&cx, &result, "get_me_response".to_string());

        result
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
}
