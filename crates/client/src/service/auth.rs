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
    utils::{MetadataInjector, Method, Metrics},
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

    pub async fn register(
        &self,
        request_data: RegisterDomainRequest,
    ) -> Result<ApiResponseRegister, ErrorResponse> {
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

    pub async fn get_me(&self, id: i32) -> Result<ApiResponseGetMe, ErrorResponse> {
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

        let mut client = self.client.lock().await;
        let mut request = Request::new(GetMeRequest { id});

        self.inject_trace_context(&cx, &mut request);

        let result = client.get_me(request).await;
        self.add_completion_event(&cx, &result, "get_me_response".to_string());

        result
            .map(|resp| resp.into_inner())
            .map_err(|status| ErrorResponse {
                status: status.code().to_string(),
                message: status.message().to_string(),
            })
    }
}
