use crate::{
    abstract_trait::{
        DynAuthService, DynCategoryService, DynCommentService, DynPostsService, DynUserService,
    },
    service::{AuthService, CategoryService, CommentService, PostsService, UserService},
};
use genproto::{
    auth::auth_service_client::AuthServiceClient,
    category::category_service_client::CategoryServiceClient,
    comment::comment_service_client::CommentServiceClient,
    post::posts_service_client::PostsServiceClient, user::user_service_client::UserServiceClient,
};
use shared::{abstract_trait::DynFileService, service::FileService, utils::Metrics};
use std::sync::Arc;
use tokio::sync::Mutex;
use tonic::transport::Channel;

#[derive(Clone)]
pub struct DependenciesInject {
    pub category_service: DynCategoryService,
    pub post_service: DynPostsService,
    pub comment_service: DynCommentService,
    pub user_service: DynUserService,
    pub auth_service: DynAuthService,
    pub file_service: DynFileService,
}

impl std::fmt::Debug for DependenciesInject {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DependenciesInject")
            .field("category_service", &"DynCategoryService")
            .field("post_service", &"DynPostsService")
            .field("comment_service", &"DynCommentService")
            .field("user_service", &"DynUserService")
            .field("auth_service", &"DynAuthService")
            .finish()
    }
}

impl DependenciesInject {
    pub async fn new(
        auth_client: Arc<Mutex<AuthServiceClient<Channel>>>,
        user_client: Arc<Mutex<UserServiceClient<Channel>>>,
        category_client: Arc<Mutex<CategoryServiceClient<Channel>>>,
        post_client: Arc<Mutex<PostsServiceClient<Channel>>>,
        comment_client: Arc<Mutex<CommentServiceClient<Channel>>>,
        metrics: Arc<Mutex<Metrics>>,
    ) -> Self {
        let auth_service: DynAuthService =
            Arc::new(AuthService::new(auth_client, metrics.clone()).await);
        let user_service: DynUserService =
            Arc::new(UserService::new(user_client, metrics.clone()).await);

        let category_service: DynCategoryService =
            Arc::new(CategoryService::new(category_client, metrics.clone()).await);
        let post_service: DynPostsService =
            Arc::new(PostsService::new(post_client, metrics.clone()).await);
        let comment_service: DynCommentService =
            Arc::new(CommentService::new(comment_client, metrics.clone()).await);

        let file_service: DynFileService = Arc::new(FileService::default());

        Self {
            category_service,
            post_service,
            comment_service,
            user_service,
            auth_service,
            file_service,
        }
    }
}
