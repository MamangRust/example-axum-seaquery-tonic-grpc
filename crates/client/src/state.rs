use prometheus_client::{metrics::family::Family, registry::Registry};
use std::sync::Arc;
use tokio::sync::Mutex;

use shared::{config::JwtConfig, utils::Metrics};



#[derive(Debug)]
pub struct AppState {
    pub registry: Arc<Registry>,
    pub jwt_config: JwtConfig,
    pub metrics: Arc<Mutex<Metrics>>,
}