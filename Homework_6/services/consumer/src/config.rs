use scylla::statement::Consistency;
use std::net::SocketAddr;

#[derive(Clone, Debug)]
pub struct Config {
    pub bind_addr: SocketAddr,
    pub kafka_bootstrap: String,
    pub warehouse_events_topic: String,
    pub warehouse_events_dlq_topic: String,
    pub consumer_group: String,
    pub schema_registry_url: String,
    pub cassandra_nodes: Vec<String>,
    pub read_consistency: Consistency,
}

impl Config {
    pub fn from_env() -> Result<Self, String> {
        let port: u16 = std::env::var("CONSUMER_PORT")
            .unwrap_or_else(|_| "3000".to_string())
            .parse()
            .map_err(|_| "CONSUMER_PORT must be a valid u16")?;
        let host = std::env::var("CONSUMER_BIND").unwrap_or_else(|_| "0.0.0.0".to_string());
        let bind_addr: SocketAddr = format!("{host}:{port}")
            .parse()
            .map_err(|_| "invalid CONSUMER_BIND / CONSUMER_PORT")?;
        let kafka_bootstrap = std::env::var("KAFKA_BOOTSTRAP_SERVERS")
            .map_err(|_| "KAFKA_BOOTSTRAP_SERVERS is required".to_string())?;
        let warehouse_events_topic = std::env::var("WAREHOUSE_EVENTS_TOPIC")
            .unwrap_or_else(|_| "warehouse-events".to_string());
        let warehouse_events_dlq_topic = std::env::var("WAREHOUSE_EVENTS_DLQ_TOPIC")
            .unwrap_or_else(|_| "warehouse-events-dlq".to_string());
        let consumer_group = std::env::var("WAREHOUSE_CONSUMER_GROUP")
            .unwrap_or_else(|_| "warehouse-state-consumer".to_string());
        let schema_registry_url = std::env::var("SCHEMA_REGISTRY_URL")
            .map_err(|_| "SCHEMA_REGISTRY_URL is required".to_string())?;
        let cassandra_nodes = std::env::var("CASSANDRA_CONTACT_POINTS")
            .unwrap_or_else(|_| "cassandra-1:9042,cassandra-2:9042,cassandra-3:9042".to_string())
            .split(',')
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
            .collect::<Vec<_>>();
        if cassandra_nodes.is_empty() {
            return Err("CASSANDRA_CONTACT_POINTS must contain at least one node".to_string());
        }
        let read_consistency = match std::env::var("CASSANDRA_READ_CONSISTENCY")
            .unwrap_or_else(|_| "ONE".to_string())
            .to_uppercase()
            .as_str()
        {
            "QUORUM" => Consistency::Quorum,
            _ => Consistency::One,
        };
        Ok(Self {
            bind_addr,
            kafka_bootstrap,
            warehouse_events_topic,
            warehouse_events_dlq_topic,
            consumer_group,
            schema_registry_url,
            cassandra_nodes,
            read_consistency,
        })
    }
}
