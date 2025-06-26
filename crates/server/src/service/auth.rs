use std::sync::Arc;
use tonic::{Request, Response, Status};

use genproto::auth::{
    ApiResponseGetMe, ApiResponseLogin, ApiResponseRegister, GetMeRequest, LoginRequest,
    RegisterRequest, auth_service_server::AuthService,
};

use shared::{
    domain::{LoginRequest as LoginDomainRequest, RegisterRequest as RegisterDomainRequest},
    state::AppState,
};

#[derive(Debug, Clone)]
pub struct AuthServiceImpl {
    pub state: Arc<AppState>,
}

impl AuthServiceImpl {
    pub fn new(state: Arc<AppState>) -> Self {
        Self { state }
    }
}

#[tonic::async_trait]
impl AuthService for AuthServiceImpl {
    async fn login_user(
        &self,
        request: Request<LoginRequest>,
    ) -> Result<Response<ApiResponseLogin>, Status> {
        let req = request.into_inner();

        let domain_req = LoginDomainRequest {
            email: req.email,
            password: req.password,
        };

        match self
            .state
            .di_container
            .auth_service
            .login_user(&domain_req)
            .await
        {
            Ok(api_response) => {
                let reply = ApiResponseLogin {
                    status: api_response.status,
                    message: api_response.message,
                    data: api_response.data,
                };
                Ok(Response::new(reply))
            }
            Err(err) => Err(Status::unauthenticated(err.message)),
        }
    }

    async fn register_user(
        &self,
        request: Request<RegisterRequest>,
    ) -> Result<Response<ApiResponseRegister>, Status> {
        let req = request.into_inner();

        let domain_req = RegisterDomainRequest {
            firstname: req.firstname,
            lastname: req.lastname,
            email: req.email,
            password: req.password,
        };

        match self
            .state
            .di_container
            .auth_service
            .register_user(&domain_req)
            .await
        {
            Ok(api_response) => {
                let user = Some(api_response.data.into()); // assuming From<UserResponse> implemented
                let reply = ApiResponseRegister {
                    status: api_response.status,
                    message: api_response.message,
                    data: user,
                };
                Ok(Response::new(reply))
            }
            Err(err) => Err(Status::internal(err.message)),
        }
    }

    async fn get_me(
        &self,
        request: Request<GetMeRequest>,
    ) -> Result<Response<ApiResponseGetMe>, Status> {
        let req = request.into_inner();

        match self
            .state
            .di_container
            .user_service
            .find_by_id(req.id)
            .await
        {
            Ok(Some(user)) => {
                let reply = ApiResponseGetMe {
                    status: "success".into(),
                    message: "User fetched successfully".into(),
                    data: Some(user.data.into()),
                };
                Ok(Response::new(reply))
            }
            Ok(None) => Err(Status::not_found("User not found")),
            Err(err) => Err(Status::internal(err.message)),
        }
    }
}
