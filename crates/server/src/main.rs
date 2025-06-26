use axum::routing::get;
use genproto::{
    auth::auth_service_server::AuthServiceServer,
    category::category_service_server::CategoryServiceServer,
    comment::comment_service_server::CommentServiceServer,
    post::posts_service_server::PostsServiceServer, user::user_service_server::UserServiceServer,
};
use shared::{
    config::{Config, ConnectionManager},
    state::AppState,
    utils::{Telemetry, init_logger, metrics_handler},
};
use std::{error::Error, sync::Arc};
use tonic::transport::Server;

mod service;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
    dotenv::dotenv().ok();

    let mytelemetry = Telemetry::new("myserver");
    let tracer_provider = mytelemetry.init_tracer();
    let meter_provider = mytelemetry.init_meter();
    let logger_provider = mytelemetry.init_logger();
    init_logger(logger_provider.clone());

    let config = Config::init();
    let db_pool = ConnectionManager::new_pool(&config.database_url, config.run_migrations)
        .await
        .expect("Error initializing database connection pool");

    let state = Arc::new(AppState::new(db_pool, &config.jwt_secret));

    let service_auth = service::auth::AuthServiceImpl::new(state.clone());
    let service_user = service::user::UserServiceImpl::new(state.clone());
    let service_post = service::posts::PostsServiceImpl::new(state.clone());
    let service_comment = service::comment::CommentServiceImpl::new(state.clone());
    let service_category = service::category::CategoryServiceImpl::new(state.clone());

    let addr = "0.0.0.0:50051".parse()?;

    let grpc_server = tokio::spawn(async move {
        Server::builder()
            .add_service(AuthServiceServer::new(service_auth))
            .add_service(UserServiceServer::new(service_user))
            .add_service(PostsServiceServer::new(service_post))
            .add_service(CommentServiceServer::new(service_comment))
            .add_service(CategoryServiceServer::new(service_category))
            .serve(addr)
            .await
    });

    let app = axum::Router::new()
        .route("/metrics", get(metrics_handler))
        .with_state(state.clone());

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await?;

    println!("gRPC Server running on 0.0.0.0:50051");
    println!("Metrics Server running on http://0.0.0.0:8080");

    let axum_server = tokio::spawn(async move { axum::serve(listener, app).await });

    let (grpc_result, axum_result) = tokio::try_join!(grpc_server, axum_server)?;
    grpc_result?;
    axum_result?;

    let mut shutdown_errors = Vec::new();
    if let Err(e) = tracer_provider.shutdown() {
        shutdown_errors.push(format!("tracer provider: {}", e));
    }
    if let Err(e) = meter_provider.shutdown() {
        shutdown_errors.push(format!("meter provider: {}", e));
    }
    if let Err(e) = logger_provider.shutdown() {
        shutdown_errors.push(format!("logger provider: {}", e));
    }
    if !shutdown_errors.is_empty() {
        return Err(format!(
            "Failed to shutdown providers:\n{}",
            shutdown_errors.join("\n")
        )
        .into());
    }
    Ok(())
}
