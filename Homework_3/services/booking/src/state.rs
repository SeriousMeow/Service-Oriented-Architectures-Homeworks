use deadpool_postgres::{Config as PgConfig, ManagerConfig, Pool, RecyclingMethod, Runtime};
use tokio_postgres::NoTls;

use crate::grpc::flight_client::FlightClientConfig;

#[derive(Clone)]
pub struct AppState {
    pub db: Pool,
    pub flight_client: FlightClientConfig,
}

impl AppState {
    pub fn from_env() -> anyhow::Result<Self> {
        let host = std::env::var("POSTGRES_HOST").unwrap_or_else(|_| "localhost".to_string());
        let port = std::env::var("POSTGRES_PORT").unwrap_or_else(|_| "5432".to_string());
        let user = std::env::var("POSTGRES_USER").unwrap_or_else(|_| "user".to_string());
        let password =
            std::env::var("POSTGRES_PASSWORD").unwrap_or_else(|_| "password".to_string());
        let database = std::env::var("POSTGRES_DB").unwrap_or_else(|_| "booking".to_string());

        let mut config = PgConfig::new();
        config.host = Some(host);
        config.port = Some(port.parse()?);
        config.user = Some(user);
        config.password = Some(password);
        config.dbname = Some(database);
        config.manager = Some(ManagerConfig {
            recycling_method: RecyclingMethod::Fast,
        });
        config.pool = Some(deadpool_postgres::PoolConfig {
            max_size: 16,
            ..Default::default()
        });

        let db = config.create_pool(Some(Runtime::Tokio1), NoTls)?;
        let flight_client = FlightClientConfig::from_env()?;
        Ok(Self { db, flight_client })
    }
}
