#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub jwt_secret: String,
    pub run_migrations: bool,
    pub port: u16,
}

impl Config {
    pub fn init() -> Config {
        let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
        let jwt_secret = std::env::var("JWT_SECRET").expect("JWT_SECRET must be set");

        let run_migrations_str =
            std::env::var("RUN_MIGRATIONS").expect("RUN_MIGRATIONS must be set");
        let port_str = std::env::var("PORT").expect("PORT must be set");


        let run_migrations = match run_migrations_str.as_str() {
            "true" => true,
            "false" => false,
            _ => panic!("RUN_MIGRATIONS must be either 'true' or 'false'"),
        };

        let port = port_str.parse().expect("Invalid value for PORT");

        Config { database_url, jwt_secret, run_migrations, port }
 
    }
}
