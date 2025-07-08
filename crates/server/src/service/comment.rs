use genproto::api::ApiResponseEmpty;
use genproto::comment::Empty;
use genproto::comment::{
    ApiResponseComment, ApiResponsesComment, CreateCommentRequest as ProtoCreateCommentRequest,
    FindCommentRequest, UpdateCommentRequest as ProtoUpdateCommentRequest,
    comment_service_server::CommentService,
};

use shared::{
    domain::{
        CreateCommentRequest as SharedCreateCommentRequest,
        UpdateCommentRequest as SharedUpdateCommentRequest,
    },
    state::AppState,
};
use std::sync::Arc;
use tonic::{Request, Response, Status};
use tracing::{error, info};

pub struct CommentServiceImpl {
    pub state: Arc<AppState>,
}

impl CommentServiceImpl {
    pub fn new(state: Arc<AppState>) -> Self {
        Self { state }
    }
}

#[tonic::async_trait]
impl CommentService for CommentServiceImpl {
    async fn get_comments(
        &self,
        _request: Request<Empty>,
    ) -> Result<Response<ApiResponsesComment>, Status> {
        info!("Getting comments");

        match self.state.di_container.comment_service.get_comments().await {
            Ok(api_response) => {
                let comments: Vec<_> = api_response.data.into_iter().map(Into::into).collect();

                Ok(Response::new(ApiResponsesComment {
                    status: api_response.status,
                    message: api_response.message,
                    data: comments,
                }))
            }
            Err(err) => {
                error!("Failed to get comments: {}", err.message);
                Err(Status::internal(err.message))
            }
        }
    }

    async fn get_comment(
        &self,
        request: Request<FindCommentRequest>,
    ) -> Result<Response<ApiResponseComment>, Status> {
        let id = request.into_inner().id;

        match self
            .state
            .di_container
            .comment_service
            .get_comment(id)
            .await
        {
            Ok(Some(comment)) => {
                let reply = ApiResponseComment {
                    status: "success".into(),
                    message: "Comment fetched successfully".into(),
                    data: Some(comment.data.into()),
                };
                Ok(Response::new(reply))
            }
            Ok(None) => Err(Status::not_found("Comment not found")),
            Err(err) => Err(Status::internal(err.message)),
        }
    }

    async fn create_comment(
        &self,
        request: Request<ProtoCreateCommentRequest>,
    ) -> Result<Response<ApiResponseComment>, Status> {
        let req = request.get_ref();

        let body = SharedCreateCommentRequest {
            id_post_comment: req.id_post_comment,
            user_name_comment: req.user_name_comment.clone(),
            comment: req.comment.clone(),
        };

        match self
            .state
            .di_container
            .comment_service
            .create_comment(&body)
            .await
        {
            Ok(comment) => Ok(Response::new(ApiResponseComment {
                status: comment.status,
                message: comment.message,
                data: Some(comment.data.into()),
            })),
            Err(err) => Err(Status::internal(err.message)),
        }
    }

    async fn update_comment(
        &self,
        request: Request<ProtoUpdateCommentRequest>,
    ) -> Result<Response<ApiResponseComment>, Status> {
        let req = request.get_ref();

        let body = SharedUpdateCommentRequest {
            id_post_comment: req.id_post_comment,
            user_name_comment: req.user_name_comment.clone(),
            comment: req.comment.clone(),
        };

        match self
            .state
            .di_container
            .comment_service
            .update_comment(&body)
            .await
        {
            Ok(Some(comment)) => Ok(Response::new(ApiResponseComment {
                status: comment.status,
                message: comment.message,
                data: Some(comment.data.into()),
            })),
            Ok(None) => Err(Status::not_found("Comment not found")),
            Err(err) => Err(Status::internal(err.message)),
        }
    }

    async fn delete_comment(
        &self,
        request: Request<FindCommentRequest>,
    ) -> Result<Response<ApiResponseEmpty>, Status> {
        let id = request.into_inner().id;

        match self
            .state
            .di_container
            .comment_service
            .delete_comment(id)
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
