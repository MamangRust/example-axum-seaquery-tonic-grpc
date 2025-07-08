use crate::{
    abstract_trait::{
        DynAuthService, DynCategoryService, DynCommentService, DynPostsService, DynUserService,
    },
    service::{
        AuthService, CategoryService, CommentService, GrpcClients, PostsService, UserService,
    },
};

use anyhow::Result;
use prometheus_client::registry::Registry;
use shared::{abstract_trait::DynFileService, service::FileService, utils::Metrics};
use std::sync::Arc;
use tokio::sync::Mutex;

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
        clients: GrpcClients,
        metrics: Arc<Mutex<Metrics>>,
        registry: &mut Registry,
    ) -> Result<Self> {
        let auth_service: DynAuthService =
            Arc::new(AuthService::new(clients.auth, metrics.clone(), registry).await);
        let user_service: DynUserService =
            Arc::new(UserService::new(clients.user, metrics.clone(), registry).await);
        let category_service: DynCategoryService =
            Arc::new(CategoryService::new(clients.category, metrics.clone(), registry).await);
        let post_service: DynPostsService =
            Arc::new(PostsService::new(clients.post, metrics.clone(), registry).await);
        let comment_service: DynCommentService =
            Arc::new(CommentService::new(clients.comment, metrics.clone(), registry).await);
        let file_service: DynFileService = Arc::new(FileService::default());

        Ok(Self {
            category_service,
            post_service,
            comment_service,
            user_service,
            auth_service,
            file_service,
        })
    }
}
