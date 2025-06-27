use std::sync::Arc;
use tokio::sync::Mutex;

use crate::{
    abstract_trait::{
        DynAuthService, DynCategoryRepository, DynCategoryService, DynCommentRepository,
        DynCommentService, DynFileService, DynPostsRepository, DynPostsService, DynUserRepository,
        DynUserService,
    },
    config::{ConnectionPool, Hashing, JwtConfig},
    repository::{CategoryRepository, CommentRepository, PostRepository, UserRepository},
    service::{
        AuthService, CategoryService, CommentService, FileService, PostService, UserService,
    },
    utils::Metrics,
};

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
            .field("file_service", &"DynFileService")
            .finish()
    }
}

impl DependenciesInject {
    pub async fn new(
        pool: ConnectionPool,
        hashing: Hashing,
        jwt_config: JwtConfig,
        metrics: Arc<Mutex<Metrics>>,
    ) -> Self {
        let category_repository =
            Arc::new(CategoryRepository::new(pool.clone())) as DynCategoryRepository;
        let post_repository = Arc::new(PostRepository::new(pool.clone())) as DynPostsRepository;
        let comment_repository =
            Arc::new(CommentRepository::new(pool.clone())) as DynCommentRepository;
        let user_repository = Arc::new(UserRepository::new(pool)) as DynUserRepository;

        let category_service =
            Arc::new(CategoryService::new(category_repository, metrics.clone()).await)
                as DynCategoryService;

        let post_service =
            Arc::new(PostService::new(post_repository, metrics.clone()).await) as DynPostsService;

        let comment_service =
            Arc::new(CommentService::new(comment_repository, metrics.clone()).await)
                as DynCommentService;

        let user_service =
            Arc::new(UserService::new(user_repository.clone(), metrics.clone()).await)
                as DynUserService;

        let auth_service =
            Arc::new(AuthService::new(user_repository, hashing, jwt_config, metrics.clone()).await)
                as DynAuthService;

        let file_service = Arc::new(FileService::default()) as DynFileService;

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
