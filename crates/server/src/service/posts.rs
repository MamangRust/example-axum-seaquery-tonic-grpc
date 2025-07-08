use genproto::api::ApiResponseEmpty;
use genproto::post::{
    ApiResponsePost, ApiResponsePostRelation, ApiResponsePostsPaginated, CreatePostRequest,
    FindAllPostRequest, FindPostRequest, UpdatePostRequest, posts_service_server::PostsService,
};
use shared::{
    domain::{
        CreatePostRequest as SharedCreatePostRequest,
        FindAllPostRequest as SharedFindAllPostRequest,
        UpdatePostRequest as SharedUpdatePostRequest,
    },
    state::AppState,
};
use std::sync::Arc;
use tonic::{Request, Response, Status};
use tracing::{error, info};

pub struct PostsServiceImpl {
    pub state: Arc<AppState>,
}

impl PostsServiceImpl {
    pub fn new(state: Arc<AppState>) -> Self {
        Self { state }
    }
}

#[tonic::async_trait]
impl PostsService for PostsServiceImpl {
    async fn find_all_posts(
        &self,
        request: Request<FindAllPostRequest>,
    ) -> Result<Response<ApiResponsePostsPaginated>, Status> {
        info!("Getting all posts");

        let req = request.get_ref();

        let myrequest = SharedFindAllPostRequest {
            page: req.page,
            page_size: req.page_size,
            search: req.search.clone(),
        };

        match self
            .state
            .di_container
            .post_service
            .get_all_posts(myrequest)
            .await
        {
            Ok(api_response) => {
                let posts: Vec<_> = api_response.data.into_iter().map(Into::into).collect();

                Ok(Response::new(ApiResponsePostsPaginated {
                    status: api_response.status,
                    message: api_response.message,
                    data: posts,
                    pagination: Some(api_response.pagination.into()),
                }))
            }
            Err(err) => {
                error!("Failed to get posts: {}", err.message);
                Err(Status::internal(err.message))
            }
        }
    }

    async fn find_post(
        &self,
        request: Request<FindPostRequest>,
    ) -> Result<Response<ApiResponsePost>, Status> {
        let post_id = request.into_inner().post_id;

        match self.state.di_container.post_service.get_post(post_id).await {
            Ok(Some(post)) => {
                let reply = ApiResponsePost {
                    status: "success".into(),
                    message: "Post fetched successfully".into(),
                    data: Some(post.data.into()),
                };
                Ok(Response::new(reply))
            }
            Ok(None) => Err(Status::not_found("Post not found")),
            Err(err) => Err(Status::internal(err.message)),
        }
    }

    async fn find_post_relation(
        &self,
        request: Request<FindPostRequest>,
    ) -> Result<Response<ApiResponsePostRelation>, Status> {
        let post_id = request.into_inner().post_id;

        match self
            .state
            .di_container
            .post_service
            .get_post_relation(post_id)
            .await
        {
            Ok(post_relation) => Ok(Response::new(ApiResponsePostRelation {
                status: post_relation.status,
                message: post_relation.message,
                data: Some(post_relation.data.into()),
            })),
            Err(err) => Err(Status::internal(err.message)),
        }
    }

    async fn create_post(
        &self,
        request: Request<CreatePostRequest>,
    ) -> Result<Response<ApiResponsePost>, Status> {
        let req = request.get_ref();

        let body = SharedCreatePostRequest {
            title: req.title.clone(),
            body: req.body.clone(),
            file: req.file.clone(),
            category_id: req.category_id,
            user_id: req.user_id,
            user_name: req.user_name.clone(),
        };

        match self
            .state
            .di_container
            .post_service
            .create_post(&body)
            .await
        {
            Ok(post) => Ok(Response::new(ApiResponsePost {
                status: post.status,
                message: post.message,
                data: Some(post.data.into()),
            })),
            Err(err) => Err(Status::internal(err.message)),
        }
    }

    async fn update_post(
        &self,
        request: Request<UpdatePostRequest>,
    ) -> Result<Response<ApiResponsePost>, Status> {
        let req = request.get_ref();

        let body = SharedUpdatePostRequest {
            post_id: req.post_id,
            title: req.title.clone(),
            body: req.body.clone(),
            file: req.file.clone(),
            category_id: req.category_id,
            user_id: req.user_id,
            user_name: req.user_name.clone(),
        };

        match self
            .state
            .di_container
            .post_service
            .update_post(&body)
            .await
        {
            Ok(post) => Ok(Response::new(ApiResponsePost {
                status: post.status,
                message: post.message,
                data: Some(post.data.into()),
            })),
            Err(err) => Err(Status::internal(err.message)),
        }
    }

    async fn delete_post(
        &self,
        request: Request<FindPostRequest>,
    ) -> Result<Response<ApiResponseEmpty>, Status> {
        let post_id = request.into_inner().post_id;

        match self
            .state
            .di_container
            .post_service
            .delete_post(post_id)
            .await
        {
            Ok(result) => Ok(Response::new(ApiResponseEmpty {
                status: result.status,
                message: result.message,
            })),
            Err(err) => Err(Status::internal(err.message)),
        }
    }
}
