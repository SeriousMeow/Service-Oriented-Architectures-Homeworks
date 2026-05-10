use std::time::Duration;

#[derive(Debug, Clone)]
pub enum ProducerMode {
    Http,
    Generator,
    Both,
}

impl ProducerMode {
    pub fn from_env() -> Self {
        match std::env::var("PRODUCER_MODE")
            .unwrap_or_else(|_| "http".to_string())
            .to_ascii_lowercase()
            .as_str()
        {
            "generator" => ProducerMode::Generator,
            "both" => ProducerMode::Both,
            _ => ProducerMode::Http,
        }
    }

    pub fn run_generator(&self) -> bool {
        matches!(self, ProducerMode::Generator | ProducerMode::Both)
    }
}

#[derive(Debug, Clone)]
pub struct Config {
    pub mode: ProducerMode,
    pub bind_addr: String,
    pub kafka_bootstrap: String,
    pub schema_registry_url: String,
    pub warehouse_events_topic: String,
    pub generator_interval: Duration,
    pub load_default_count: usize,
}

impl Config {
    pub fn from_env() -> Result<Self, String> {
        let port = std::env::var("PRODUCER_PORT").unwrap_or_else(|_| "8080".to_string());
        let bind_addr = format!("0.0.0.0:{port}");
        let kafka_bootstrap = std::env::var("KAFKA_BOOTSTRAP_SERVERS")
            .map_err(|_| "KAFKA_BOOTSTRAP_SERVERS is required".to_string())?;
        let schema_registry_url = std::env::var("SCHEMA_REGISTRY_URL")
            .map_err(|_| "SCHEMA_REGISTRY_URL is required".to_string())?;
        let warehouse_events_topic = std::env::var("WAREHOUSE_EVENTS_TOPIC")
            .unwrap_or_else(|_| "warehouse-events".to_string());
        let gen_ms: u64 = std::env::var("GENERATOR_INTERVAL_MS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(700);
        let load_default_count: usize = std::env::var("LOAD_DEFAULT_COUNT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(50);
        Ok(Config {
            mode: ProducerMode::from_env(),
            bind_addr,
            kafka_bootstrap,
            schema_registry_url,
            warehouse_events_topic,
            generator_interval: Duration::from_millis(gen_ms.max(10)),
            load_default_count: load_default_count.max(1),
        })
    }
}
