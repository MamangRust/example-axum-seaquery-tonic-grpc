use genproto::auth::{
    ApiResponseGetMe, ApiResponseLogin, ApiResponseRegister, GetMeRequest, LoginRequest,
    RegisterRequest, auth_service_client::AuthServiceClient,
};
use opentelemetry::{
    Context, KeyValue,
    global::{self, BoxedTracer},
    trace::{SpanKind, TraceContextExt, Tracer},
};
use shared::{
    domain::{
        ErrorResponse, LoginRequest as LoginDomainRequest, RegisterRequest as RegisterDomainRequest,
    },
    utils::{MetadataInjector, Metrics},
};
use std::sync::Arc;
use tokio::sync::Mutex;
use tonic::{Request, Status, transport::Channel};

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
        global::tracer("auth-service")
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

    pub async fn register(
        &self,
        request_data: RegisterDomainRequest,
    ) -> Result<ApiResponseRegister, ErrorResponse> {
        let tracer = self.get_tracer();
        let span = tracer.start("register_user");
        let cx = Context::current_with_span(span);

        let mut request = Request::new(request_data.clone());
        self.inject_trace_context(&cx, &mut request);

        let myrequest = RegisterRequest{
            firstname: request_data.firstname.clone(),
            lastname: request_data.lastname.clone(),
            email: request_data.email.clone(),
            password: request_data.password.clone(),
        };

        let result = {
            let mut client = self.client.lock().await;
            client.register_user(myrequest).await
        };

        self.add_completion_event(&cx, &result, "register_user_response".to_string());

        result
            .map(|resp| resp.into_inner())
            .map_err(|status| ErrorResponse {
                status: status.code().to_string(),
                message: status.message().to_string(),
            })
    }

    pub async fn login(
        &self,
        request_data: LoginDomainRequest,
    ) -> Result<ApiResponseLogin, ErrorResponse> {
        let tracer = self.get_tracer();
        let span = tracer.start("login_user");
        let cx = Context::current_with_span(span);

        let mut request = Request::new(request_data.clone());
        self.inject_trace_context(&cx, &mut request);

        let myrequest = LoginRequest{
            email: request_data.email.clone(),
            password: request_data.password.clone(),
        };

        let result = {
            let mut client = self.client.lock().await;
            client.login_user(myrequest).await
        };

        self.add_completion_event(&cx, &result, "login_user_response".to_string());

        result
            .map(|resp| resp.into_inner())
            .map_err(|status| ErrorResponse {
                status: status.code().to_string(),
                message: status.message().to_string(),
            })
    }
}
