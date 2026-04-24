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
    pub movie_events_topic: String,
    pub schema_subject: String,
    pub generator_interval: Duration,
}

impl Config {
    pub fn from_env() -> Result<Self, String> {
        let port = std::env::var("PRODUCER_PORT").unwrap_or_else(|_| "8080".to_string());
        let bind_addr = format!("0.0.0.0:{port}");
        let kafka_bootstrap = std::env::var("KAFKA_BOOTSTRAP_SERVERS")
            .map_err(|_| "KAFKA_BOOTSTRAP_SERVERS is required")?;
        let schema_registry_url = std::env::var("SCHEMA_REGISTRY_URL")
            .map_err(|_| "SCHEMA_REGISTRY_URL is required")?;
        let movie_events_topic =
            std::env::var("MOVIE_EVENTS_TOPIC").unwrap_or_else(|_| "movie-events".to_string());
        let schema_subject =
            std::env::var("SCHEMA_REGISTRY_SUBJECT").unwrap_or_else(|_| "movie-events-value".to_string());
        let gen_ms: u64 = std::env::var("GENERATOR_INTERVAL_MS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(750);
        Ok(Config {
            mode: ProducerMode::from_env(),
            bind_addr,
            kafka_bootstrap,
            schema_registry_url,
            movie_events_topic,
            schema_subject,
            generator_interval: Duration::from_millis(gen_ms.max(50)),
        })
    }
}
