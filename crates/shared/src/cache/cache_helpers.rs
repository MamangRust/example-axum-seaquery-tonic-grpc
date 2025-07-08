use redis::AsyncCommands;
use serde::{Serialize, de::DeserializeOwned};
use std::sync::Arc;
use tokio::time::Duration;
use tracing::{debug, error};

#[derive(Clone)]
pub struct CacheStore {
    pub redis: Arc<redis::Client>,
}

impl CacheStore {
    pub fn new(redis: redis::Client) -> Self {
        Self {
            redis: Arc::new(redis),
        }
    }

    pub async fn get_from_cache<T>(&self, key: &str) -> Option<T>
    where
        T: DeserializeOwned,
    {
        let mut conn = match self.redis.get_multiplexed_async_connection().await {
            Ok(c) => c,
            Err(e) => {
                error!("Failed to get Redis connection: {:?}", e);
                return None;
            }
        };

        let result: redis::RedisResult<String> = conn.get(key).await;
        match result {
            Ok(data) => match serde_json::from_str::<T>(&data) {
                Ok(parsed) => Some(parsed),
                Err(e) => {
                    error!("Failed to deserialize cached value: {:?}", e);
                    None
                }
            },
            Err(e) => {
                error!("Redis get error for key {}: {:?}", key, e);
                None
            }
        }
    }

    pub async fn set_to_cache<T>(&self, key: &str, data: &T, expiration: Duration)
    where
        T: Serialize,
    {
        let json_data = match serde_json::to_string(data) {
            Ok(json) => json,
            Err(e) => {
                error!("Failed to serialize data to JSON: {:?}", e);
                return;
            }
        };

        let mut conn = match self.redis.get_multiplexed_async_connection().await {
            Ok(c) => c,
            Err(e) => {
                error!("Failed to get Redis connection: {:?}", e);
                return;
            }
        };

        let result: redis::RedisResult<()> =
            conn.set_ex(key, json_data, expiration.as_secs()).await;

        match result {
            Ok(_) => debug!("Cached data under key {} with TTL {:?}", key, expiration),
            Err(e) => error!("Failed to set cache for key {}: {:?}", key, e),
        }
    }

    pub async fn delete_from_cache(&self, key: &str) {
        let mut conn = match self.redis.get_multiplexed_async_connection().await {
            Ok(c) => c,
            Err(e) => {
                error!("Failed to get Redis connection: {:?}", e);
                return;
            }
        };

        if let Err(e) = conn.del::<_, ()>(key).await {
            error!("Failed to delete key {}: {:?}", key, e);
        }
    }
}
