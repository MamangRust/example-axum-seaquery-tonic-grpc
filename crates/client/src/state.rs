use genproto::{
    auth::auth_service_client::AuthServiceClient,
    category::category_service_client::CategoryServiceClient,
    comment::comment_service_client::CommentServiceClient,
    post::posts_service_client::PostsServiceClient, user::user_service_client::UserServiceClient,
};
use prometheus_client::{metrics::family::Family, registry::Registry};
use std::sync::Arc;
use tokio::sync::Mutex;
use tonic::transport::Channel;

use shared::{
    config::JwtConfig,
    utils::{Metrics, SystemMetrics, run_metrics_collector},
};

use crate::di::DependenciesInject;

#[derive(Debug)]
pub struct AppState {
    pub registry: Arc<Registry>,
    pub jwt_config: JwtConfig,
    pub metrics: Arc<Mutex<Metrics>>,
    pub di_container: DependenciesInject,
    pub system_metrics: Arc<SystemMetrics>,
}

impl AppState {
    pub async fn new(jwt_secret: &str) -> Self {
        let jwt_config = JwtConfig::new(jwt_secret);

        let requests = Family::default();
        let mut registry = Registry::default();

        registry.register(
            "server_http_requests_client",
            "Total number of HTTP requests",
            requests.clone(),
        );

        let metrics = Arc::new(Mutex::new(Metrics { requests }));
        let system_metrics = Arc::new(SystemMetrics::new());

        system_metrics.register(&mut registry);

        let registry = Arc::new(registry);

        tokio::spawn(run_metrics_collector(system_metrics.clone()));

        let channel = Channel::from_static("http://blog-server:50051")
            .connect()
            .await
            .expect("Failed to connect to gRPC server");

        let auth_client = Arc::new(Mutex::new(AuthServiceClient::new(channel.clone())));
        let user_client = Arc::new(Mutex::new(UserServiceClient::new(channel.clone())));
        let category_client = Arc::new(Mutex::new(CategoryServiceClient::new(channel.clone())));
        let post_client = Arc::new(Mutex::new(PostsServiceClient::new(channel.clone())));
        let comment_client = Arc::new(Mutex::new(CommentServiceClient::new(channel.clone())));

        let di_container = DependenciesInject::new(
            auth_client.clone(),
            user_client.clone(),
            category_client.clone(),
            post_client.clone(),
            comment_client.clone(),
            metrics.clone(),
        );

        Self {
            registry,
            di_container,
            jwt_config,
            metrics,
            system_metrics,
        }
    }
}
