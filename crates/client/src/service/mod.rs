mod auth;
mod category;
mod comment;
mod posts;
mod user;

pub use self::auth::AuthService;
pub use self::category::CategoryService;
pub use self::comment::CommentService;
pub use self::posts::PostsService;
pub use self::user::UserService;

use std::sync::Arc;
use tokio::sync::Mutex;
use tonic::transport::Channel;

use genproto::{
    auth::auth_service_client::AuthServiceClient,
    category::category_service_client::CategoryServiceClient,
    comment::comment_service_client::CommentServiceClient,
    post::posts_service_client::PostsServiceClient, user::user_service_client::UserServiceClient,
};

#[derive(Clone)]
pub struct GrpcClients {
    pub auth: Arc<Mutex<AuthServiceClient<Channel>>>,
    pub user: Arc<Mutex<UserServiceClient<Channel>>>,
    pub category: Arc<Mutex<CategoryServiceClient<Channel>>>,
    pub post: Arc<Mutex<PostsServiceClient<Channel>>>,
    pub comment: Arc<Mutex<CommentServiceClient<Channel>>>,
}

impl GrpcClients {
    pub async fn init(channel: Channel) -> Self {
        Self {
            auth: Arc::new(Mutex::new(AuthServiceClient::new(channel.clone()))),
            user: Arc::new(Mutex::new(UserServiceClient::new(channel.clone()))),
            category: Arc::new(Mutex::new(CategoryServiceClient::new(channel.clone()))),
            post: Arc::new(Mutex::new(PostsServiceClient::new(channel.clone()))),
            comment: Arc::new(Mutex::new(CommentServiceClient::new(channel))),
        }
    }
}
