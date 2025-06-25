use genproto::{auth::auth_service_server::AuthServiceServer, category::category_service_server::CategoryServiceServer, comment::comment_service_server::CommentServiceServer, post::posts_service_server::PostsServiceServer, user::user_service_server::UserServiceServer};
use shared::{config::{Config, ConnectionManager}, state::AppState, utils::{init_logger, Telemetry}};
use std::{error::Error, sync::Arc};
use tonic::transport::Server;

mod service;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
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


   Server::builder()
        .add_service(AuthServiceServer::new(service_auth))
        .add_service(UserServiceServer::new(service_user))
        .add_service(PostsServiceServer::new(service_post))
        .add_service(CommentServiceServer::new(service_comment))
        .add_service(CategoryServiceServer::new(service_category))
        .serve(addr)
        .await?;

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
