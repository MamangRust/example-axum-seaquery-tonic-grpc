use prometheus_client::{metrics::family::Family, registry::Registry};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::{
    config::{ConnectionPool, Hashing, JwtConfig},
    utils::{DependenciesInject, Metrics, SystemMetrics},
};

#[derive(Clone, Debug)]
pub struct AppState {
    pub registry: Arc<Registry>,
    pub di_container: DependenciesInject,
    pub jwt_config: JwtConfig,
    pub metrics: Arc<Mutex<Metrics>>,
    pub system_metrics: Arc<SystemMetrics>,
}

impl AppState {
    pub fn new(pool: ConnectionPool, jwt_secret: &str, ) -> Self {
        let jwt_config = JwtConfig::new(jwt_secret);
        let hashing = Hashing::new();

        let registry = Arc::new(prometheus_client::registry::Registry::default());
        let metrics = Arc::new(Mutex::new(Metrics {
            requests: Family::default(),
        }));
        let system_metrics = Arc::new(SystemMetrics::new());

         let di_container = DependenciesInject::new(pool, hashing, jwt_config.clone(), metrics.clone());

        Self {
            registry,
            di_container,
            jwt_config,
            metrics,
            system_metrics,
        }
    }
}
