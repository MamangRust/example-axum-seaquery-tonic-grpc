use dotenv::dotenv;
use seaquery_client::{handler::AppRouter, state::AppState};
use shared::{
    config::Config,
    utils::{Telemetry, init_logger},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();

    let mytelemetry = Telemetry::new("myclient");
    let tracer_provider = mytelemetry.init_tracer();
    let meter_provider = mytelemetry.init_meter();
    let logger_provider = mytelemetry.init_logger();

    init_logger(logger_provider.clone());

    let config = Config::init();

    let port = config.port;

    let state = AppState::new(&config.jwt_secret).await;

    println!("🚀 Server started successfully");

    AppRouter::serve(port, state).await?;

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
