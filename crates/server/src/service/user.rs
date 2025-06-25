use genproto::api::ApiResponseEmpty;
use genproto::user::{
    ApiResponseUserResponse, ApiResponsesUserResponse, CreateUserRequest, DeleteUserRequest,
    FindAllUserRequest, FindUserByIdRequest, UpdateUserRequest,
    user_service_server::UserService,
};
use shared::{
    domain::{
        CreateUserRequest as SharedCreateUserRequest,
        FindAllUserRequest as SharedFindAllUserRequest,
        UpdateUserRequest as SharedUpdateUserRequest,
    },
    state::AppState,
};
use std::sync::Arc;
use tonic::{Request, Response, Status};

pub struct UserServiceImpl {
    pub state: Arc<AppState>,
}

impl UserServiceImpl {
    pub fn new(state: Arc<AppState>) -> Self {
        Self { state }
    }
}

#[tonic::async_trait]
impl UserService for UserServiceImpl {
    async fn create_user(
        &self,
        request: Request<CreateUserRequest>,
    ) -> Result<Response<ApiResponseUserResponse>, Status> {
        let myrequest = SharedCreateUserRequest {
            firstname: request.get_ref().firstname.clone(),
            lastname: request.get_ref().lastname.clone(),
            email: request.get_ref().email.clone(),
            password: request.get_ref().password.clone(),
        };

        match self
            .state
            .di_container
            .user_service
            .create_user(&myrequest)
            .await
        {
            Ok(user) => Ok(Response::new(ApiResponseUserResponse {
                status: user.status,
                message: user.message,
                data: Some(user.data.into()),
            })),
            Err(err) => Err(Status::internal(err.message)),
        }
    }

    async fn find_all_users(
        &self,
        request: Request<FindAllUserRequest>,
    ) -> Result<Response<ApiResponsesUserResponse>, Status> {
        let req = request.get_ref();

        let myrequest = SharedFindAllUserRequest {
            page: req.page,
            page_size: req.page_size,
            search: req.search.clone(),
        };

        match self
            .state
            .di_container
            .user_service
            .get_users(myrequest)
            .await
        {
            Ok(api_response) => {
                let user_responses: Vec<_> =
                    api_response.data.into_iter().map(Into::into).collect();

                Ok(Response::new(ApiResponsesUserResponse {
                    status: api_response.status,
                    message: api_response.message,
                    data: user_responses,
                    pagination: Some(api_response.pagination.into()),
                }))
            }
            Err(err) => {
                tracing::error!("Failed to fetch users: {}", err);
                Err(Status::internal("Failed to fetch users"))
            }
        }
    }

    async fn find_by_id(
        &self,
        request: Request<FindUserByIdRequest>,
    ) -> Result<Response<ApiResponseUserResponse>, Status> {
        let id = request.into_inner().id;

        match self.state.di_container.user_service.find_by_id(id).await {
            Ok(Some(user)) => {
                let reply = ApiResponseUserResponse {
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

    async fn update_user(
        &self,
        request: Request<UpdateUserRequest>,
    ) -> Result<Response<ApiResponseUserResponse>, Status> {
        let req = request.get_ref();

        let body = SharedUpdateUserRequest {
            id: Some(req.id),
            firstname: Some(req.firstname.clone()),
            lastname: Some(req.lastname.clone()),
            email: Some(req.email.clone()),
            password: Some(req.password.clone()),
        };

        match self
            .state
            .di_container
            .user_service
            .update_user(&body)
            .await
        {
            Ok(Some(api_response)) => Ok(Response::new(ApiResponseUserResponse {
                status: api_response.status,
                message: api_response.message,
                data: Some(api_response.data.into()),
            })),
            Ok(None) => Err(Status::not_found("User not found")),
            Err(err) => Err(Status::internal(err.message)),
        }
    }

    async fn delete_user(
        &self,
        request: Request<DeleteUserRequest>,
    ) -> Result<Response<ApiResponseEmpty>, Status> {
        let email = request.get_ref().email.clone();

        match self
            .state
            .di_container
            .user_service
            .delete_user(email.as_str())
            .await
        {
            Ok(user) => Ok(Response::new(ApiResponseEmpty {
                status: user.status,
                message: user.message,
            })),
            Err(err) => Err(Status::internal(err.message)),
        }
    }
}
