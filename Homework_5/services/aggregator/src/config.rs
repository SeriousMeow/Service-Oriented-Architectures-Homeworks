use std::net::SocketAddr;

#[derive(Clone, Debug)]
pub struct Config {
    pub bind_addr: SocketAddr,
    pub clickhouse_url: String,
    pub clickhouse_user: String,
    pub clickhouse_password: String,
    pub aggregation_interval: std::time::Duration,
}

impl Config {
    pub fn from_env() -> Result<Self, String> {
        let port: u16 = std::env::var("AGGREGATOR_PORT")
            .unwrap_or_else(|_| "8081".to_string())
            .parse()
            .map_err(|_| "AGGREGATOR_PORT must be a valid u16")?;

        let host = std::env::var("AGGREGATOR_BIND").unwrap_or_else(|_| "0.0.0.0".to_string());
        let bind_addr: SocketAddr = format!("{host}:{port}")
            .parse()
            .map_err(|_| "invalid AGGREGATOR_BIND / AGGREGATOR_PORT")?;

        let clickhouse_url = std::env::var("CLICKHOUSE_URL")
            .unwrap_or_else(|_| "http://localhost:8123".to_string());
        let clickhouse_user = std::env::var("CLICKHOUSE_USER").unwrap_or_else(|_| "default".to_string());
        let clickhouse_password =
            std::env::var("CLICKHOUSE_PASSWORD").unwrap_or_else(|_| "".to_string());

        let sec: u64 = std::env::var("AGGREGATION_INTERVAL_SEC")
            .unwrap_or_else(|_| "300".to_string())
            .parse()
            .map_err(|_| "AGGREGATION_INTERVAL_SEC must be a positive integer")?;
        if sec == 0 {
            return Err("AGGREGATION_INTERVAL_SEC must be > 0".into());
        }

        Ok(Self {
            bind_addr,
            clickhouse_url: clickhouse_url.trim_end_matches('/').to_string(),
            clickhouse_user,
            clickhouse_password,
            aggregation_interval: std::time::Duration::from_secs(sec),
        })
    }
}
