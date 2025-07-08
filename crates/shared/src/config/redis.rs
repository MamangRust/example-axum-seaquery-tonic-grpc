use anyhow::Result;
use redis::{Client, Connection, RedisResult};

#[derive(Debug, Clone)]
pub struct RedisConfig {
    pub host: String,
    pub port: u16,
    pub db: u8,
    pub password: Option<String>,
}

#[derive(Clone)]
pub struct RedisClient {
    pub client: Client,
}

impl RedisClient {
    pub async fn new(config: &RedisConfig) -> Result<Self> {
        let url = match &config.password {
            Some(pw) => format!(
                "redis://:{}@{}:{}/{}",
                pw, config.host, config.port, config.db
            ),
            None => format!("redis://{}:{}/{}", config.host, config.port, config.db),
        };

        let client = Client::open(url)?;
        Ok(Self { client })
    }

    pub fn get_connection(&self) -> RedisResult<Connection> {
        self.client.get_connection()
    }

    pub fn ping(&self) -> Result<()> {
        let mut conn = self.get_connection()?;
        let _: () = redis::cmd("PING").query(&mut conn)?;
        Ok(())
    }
}
