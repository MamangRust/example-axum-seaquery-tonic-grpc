use std::sync::Arc;
use tokio::sync::Mutex;

use crate::{
    abstract_trait::{AuthServiceTrait, DynUserRepository},
    config::{Hashing, JwtConfig},
    domain::{
        ApiResponse, CreateUserRequest, ErrorResponse, LoginRequest, RegisterRequest, UserResponse,
    },
    utils::{AppError, MetadataInjector, Method, Metrics},
};
use async_trait::async_trait;
use opentelemetry::{
    Context, KeyValue,
    global::{self, BoxedTracer},
    trace::{SpanKind, TraceContextExt, Tracer},
};
use tonic::{Request, Status};

#[derive(Clone)]
pub struct AuthService {
    repository: DynUserRepository,
    hashing: Hashing,
    jwt_config: JwtConfig,
    metrics: Arc<Mutex<Metrics>>,
}

impl std::fmt::Debug for AuthService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AuthService")
            .field("repository", &"DynUserRepository")
            .field("hashing", &"Hashing")
            .field("jwt_config", &"JwtConfig")
            .finish()
    }
}

impl AuthService {
    pub fn new(
        repository: DynUserRepository,
        hashing: Hashing,
        jwt_config: JwtConfig,
        metrics: Arc<Mutex<Metrics>>,
    ) -> Self {
        Self {
            repository,
            hashing,
            jwt_config,
            metrics,
        }
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
}

#[async_trait]
impl AuthServiceTrait for AuthService {
    async fn register_user(
        &self,
        input: &RegisterRequest,
    ) -> Result<ApiResponse<UserResponse>, ErrorResponse> {
        self.metrics.lock().await.inc_requests(Method::Post);

        let tracer = self.get_tracer();

        let span = tracer
            .span_builder("RegisterUser")
            .with_kind(SpanKind::Client)
            .with_attributes([
                KeyValue::new("component", "auth"),
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

        let hashed_password = self
            .hashing
            .hash_password(&input.password)
            .await
            .map_err(|e| ErrorResponse::from(AppError::HashingError(e)))?;

        let request = CreateUserRequest {
            firstname: input.firstname.clone(),
            lastname: input.lastname.clone(),
            email: input.email.clone(),
            password: hashed_password,
        };

        let create_user = self
            .repository
            .create_user(&request)
            .await
            .map_err(ErrorResponse::from)?;

        self.add_completion_event(&cx, &Ok(create_user.clone()), "UserCreated".to_string());

        Ok(ApiResponse {
            status: "success".to_string(),
            message: "User registered successfully".to_string(),
            data: UserResponse::from(create_user),
        })
    }

    async fn login_user(&self, input: &LoginRequest) -> Result<ApiResponse<String>, ErrorResponse> {
        self.metrics.lock().await.inc_requests(Method::Post);

        let tracer = self.get_tracer();
        let span = tracer
            .span_builder("LoginUser")
            .with_kind(SpanKind::Client)
            .with_attributes([
                KeyValue::new("component", "auth"),
                KeyValue::new("user.email", input.email.clone()),
            ])
            .start(&tracer);
        let cx = Context::current_with_span(span);

        let mut request = Request::new(input.clone());
        self.inject_trace_context(&cx, &mut request);

        let user = self
            .repository
            .find_by_email(&input.email)
            .await
            .map_err(ErrorResponse::from)?
            .ok_or_else(|| ErrorResponse::from(AppError::NotFound("User not found".to_string())))?;

        if self
            .hashing
            .compare_password(&user.password, &input.password)
            .await
            .is_err()
        {
            return Err(ErrorResponse::from(AppError::InvalidCredentials));
        }

        let token = self
            .jwt_config
            .generate_token(user.id as i64)
            .map_err(ErrorResponse::from)?;

        self.add_completion_event(&cx, &Ok(token.clone()), "LoginSuccessful".to_string());

        Ok(ApiResponse {
            status: "success".to_string(),
            message: "Login successful".to_string(),
            data: token,
        })
    }

    fn verify_token(&self, token: &str) -> Result<i64, AppError> {
        self.jwt_config.verify_token(token)
    }
}
