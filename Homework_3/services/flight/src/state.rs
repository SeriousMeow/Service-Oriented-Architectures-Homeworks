use deadpool_postgres::{Config as PgConfig, ManagerConfig, Pool, RecyclingMethod, Runtime};
use deadpool_redis::redis;
use std::pin::Pin;
use tokio_postgres::NoTls;

#[derive(Clone)]
pub enum RedisPool {
    Single(deadpool_redis::Pool),
    Sentinel(deadpool_redis::sentinel::Pool),
}

pub enum RedisConnection {
    Single(deadpool_redis::Connection),
    Sentinel(deadpool_redis::sentinel::Connection),
}

impl redis::aio::ConnectionLike for RedisConnection {
    fn req_packed_command<'a>(
        &'a mut self,
        cmd: &'a redis::Cmd,
    ) -> redis::RedisFuture<'a, redis::Value> {
        match self {
            RedisConnection::Single(c) => redis::aio::ConnectionLike::req_packed_command(c, cmd),
            RedisConnection::Sentinel(c) => redis::aio::ConnectionLike::req_packed_command(c, cmd),
        }
    }

    fn req_packed_commands<'a>(
        &'a mut self,
        pipeline: &'a redis::Pipeline,
        offset: usize,
        count: usize,
    ) -> Pin<
        Box<
            dyn std::future::Future<Output = Result<Vec<redis::Value>, redis::RedisError>>
                + Send
                + 'a,
        >,
    > {
        match self {
            RedisConnection::Single(c) => {
                redis::aio::ConnectionLike::req_packed_commands(c, pipeline, offset, count)
            }
            RedisConnection::Sentinel(c) => {
                redis::aio::ConnectionLike::req_packed_commands(c, pipeline, offset, count)
            }
        }
    }

    fn get_db(&self) -> i64 {
        match self {
            RedisConnection::Single(c) => redis::aio::ConnectionLike::get_db(c),
            RedisConnection::Sentinel(c) => redis::aio::ConnectionLike::get_db(c),
        }
    }
}

impl RedisPool {
    pub async fn get(&self) -> Result<RedisConnection, deadpool_redis::PoolError> {
        match self {
            RedisPool::Single(p) => p.get().await.map(RedisConnection::Single),
            RedisPool::Sentinel(p) => p.get().await.map(RedisConnection::Sentinel),
        }
    }
}

#[derive(Clone)]
pub struct AppState {
    pub db: Pool,
    pub redis: RedisPool,
    pub cache_ttl_seconds: u64,
    pub service_api_key: String,
}

impl AppState {
    pub fn from_env() -> Result<Self, Box<dyn std::error::Error>> {
        let host = std::env::var("POSTGRES_HOST").unwrap_or_else(|_| "localhost".to_string());
        let port = std::env::var("POSTGRES_PORT").unwrap_or_else(|_| "5432".to_string());
        let user = std::env::var("POSTGRES_USER").unwrap_or_else(|_| "user".to_string());
        let password =
            std::env::var("POSTGRES_PASSWORD").unwrap_or_else(|_| "password".to_string());
        let database = std::env::var("POSTGRES_DB").unwrap_or_else(|_| "flight".to_string());
        let service_api_key = std::env::var("SERVICE_API_KEY")
            .map_err(|_| "SERVICE_API_KEY env var is required")?
            .trim()
            .to_string();
        if service_api_key.is_empty() {
            return Err("SERVICE_API_KEY env var must be non-empty".into());
        }

        let redis_mode = std::env::var("REDIS_MODE").unwrap_or_else(|_| "single".to_string());
        let redis_host = std::env::var("REDIS_HOST").unwrap_or_else(|_| "localhost".to_string());
        let redis_port = std::env::var("REDIS_PORT").unwrap_or_else(|_| "6379".to_string());
        let cache_ttl_seconds = std::env::var("CACHE_TTL_SECONDS")
            .unwrap_or_else(|_| "600".to_string())
            .parse::<u64>()
            .map_err(|_| "CACHE_TTL_SECONDS must be a valid integer")?;

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

        let redis = match redis_mode.trim().to_lowercase().as_str() {
            "single" => {
                let redis_config =
                    deadpool_redis::Config::from_url(format!("redis://{redis_host}:{redis_port}/"));
                RedisPool::Single(redis_config.create_pool(Some(deadpool_redis::Runtime::Tokio1))?)
            }
            "sentinel" => {
                let master_name = std::env::var("REDIS_SENTINEL_MASTER_NAME")
                    .map_err(|_| "REDIS_SENTINEL_MASTER_NAME env var is required in sentinel mode")?
                    .trim()
                    .to_string();
                if master_name.is_empty() {
                    return Err("REDIS_SENTINEL_MASTER_NAME env var must be non-empty".into());
                }

                let nodes = std::env::var("REDIS_SENTINEL_NODES")
                    .map_err(|_| "REDIS_SENTINEL_NODES env var is required in sentinel mode")?;
                let urls: Vec<String> = nodes
                    .split(',')
                    .map(|s| s.trim())
                    .filter(|s| !s.is_empty())
                    .map(|hostport| {
                        if hostport.starts_with("redis://") || hostport.starts_with("rediss://") {
                            hostport.to_string()
                        } else {
                            format!("redis://{hostport}")
                        }
                    })
                    .collect();
                if urls.is_empty() {
                    return Err("REDIS_SENTINEL_NODES must contain at least one node".into());
                }

                let mut sentinel_cfg = deadpool_redis::sentinel::Config::from_urls(
                    urls,
                    master_name,
                    deadpool_redis::sentinel::SentinelServerType::Master,
                );

                // Optional auth for the underlying redis servers (master/replicas).
                if let Ok(password) = std::env::var("REDIS_PASSWORD") {
                    let password = password.trim().to_string();
                    if !password.is_empty() {
                        sentinel_cfg = sentinel_cfg.with_node_connection_info(Some(
                            deadpool_redis::sentinel::SentinelNodeConnectionInfo {
                                tls_mode: None,
                                redis_connection_info: Some(
                                    redis::RedisConnectionInfo::default()
                                        .set_password(password)
                                        .into(),
                                ),
                            },
                        ));
                    }
                }

                RedisPool::Sentinel(
                    sentinel_cfg.create_pool(Some(deadpool_redis::sentinel::Runtime::Tokio1))?,
                )
            }
            other => return Err(format!("Unsupported REDIS_MODE: {other}").into()),
        };

        Ok(Self {
            db,
            redis,
            cache_ttl_seconds,
            service_api_key,
        })
    }
}
