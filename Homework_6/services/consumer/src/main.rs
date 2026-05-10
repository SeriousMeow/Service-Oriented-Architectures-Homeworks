mod api;
mod cassandra;
mod config;
mod consumer;
mod dlq;
mod metrics;
mod schema_registry;

use crate::api::{AppState, router};
use crate::cassandra::CassandraStore;
use crate::config::Config;
use crate::dlq::DlqPublisher;
use crate::metrics::Metrics;
use crate::schema_registry::SchemaRegistryDecoder;
use anyhow::Context;
use rdkafka::config::ClientConfig;
use rdkafka::consumer::{Consumer, StreamConsumer};
use rdkafka::producer::FutureProducer;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tokio::signal;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let cfg = Config::from_env().map_err(anyhow::Error::msg)?;
    let metrics = Arc::new(Metrics::new().map_err(anyhow::Error::msg)?);
    let cassandra = Arc::new(
        CassandraStore::connect(&cfg.cassandra_nodes, cfg.read_consistency)
            .await
            .context("cassandra init")?,
    );
    let decoder = Arc::new(SchemaRegistryDecoder::new(
        cfg.schema_registry_url.clone(),
        cfg.warehouse_events_topic.clone(),
    ));

    let consumer: StreamConsumer = ClientConfig::new()
        .set("bootstrap.servers", &cfg.kafka_bootstrap)
        .set("group.id", &cfg.consumer_group)
        .set("enable.auto.commit", "false")
        .set("auto.offset.reset", "earliest")
        .set("enable.partition.eof", "false")
        .set("session.timeout.ms", "10000")
        .set("max.poll.interval.ms", "300000")
        .create()
        .map_err(|e| anyhow::anyhow!(e.to_string()))?;
    consumer
        .subscribe(&[&cfg.warehouse_events_topic])
        .map_err(|e| anyhow::anyhow!(e.to_string()))?;
    let consumer = Arc::new(consumer);

    let dlq_producer: FutureProducer = ClientConfig::new()
        .set("bootstrap.servers", &cfg.kafka_bootstrap)
        .set("acks", "all")
        .set("enable.idempotence", "true")
        .create()
        .map_err(|e| anyhow::anyhow!(e.to_string()))?;
    let dlq = Arc::new(DlqPublisher::new(
        dlq_producer,
        cfg.warehouse_events_dlq_topic.clone(),
    ));

    let kafka_healthy = Arc::new(AtomicBool::new(true));
    let cassandra_healthy = Arc::new(AtomicBool::new(true));

    spawn_kafka_healthcheck(
        cfg.kafka_bootstrap.clone(),
        kafka_healthy.clone(),
        cfg.warehouse_events_topic.clone(),
    );
    spawn_cassandra_healthcheck(cassandra.clone(), cassandra_healthy.clone());

    let consumer_task = {
        let consumer = consumer.clone();
        let decoder = decoder.clone();
        let cassandra = cassandra.clone();
        let dlq = dlq.clone();
        let metrics = metrics.clone();
        let kafka_healthy = kafka_healthy.clone();
        tokio::spawn(async move {
            if let Err(err) =
                consumer::run(consumer, decoder, cassandra, dlq, metrics, kafka_healthy).await
            {
                tracing::error!(error = %err, "consumer loop exited");
            }
        })
    };

    let app_state = AppState {
        metrics,
        kafka_healthy,
        cassandra_healthy,
    };
    let app = router(app_state);
    let listener = tokio::net::TcpListener::bind(cfg.bind_addr).await?;
    tracing::info!(addr = %cfg.bind_addr, "warehouse-consumer listening");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    consumer_task.abort();
    Ok(())
}

fn spawn_kafka_healthcheck(bootstrap: String, health: Arc<AtomicBool>, topic: String) {
    tokio::spawn(async move {
        let check_consumer: StreamConsumer = match ClientConfig::new()
            .set("bootstrap.servers", &bootstrap)
            .set("group.id", "warehouse-consumer-health")
            .set("enable.auto.commit", "false")
            .create()
        {
            Ok(consumer) => consumer,
            Err(err) => {
                tracing::error!(error = %err, "kafka health consumer init failed");
                health.store(false, Ordering::Relaxed);
                return;
            }
        };
        loop {
            let ok = check_consumer
                .fetch_metadata(Some(&topic), Duration::from_secs(2))
                .is_ok();
            health.store(ok, Ordering::Relaxed);
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    });
}

fn spawn_cassandra_healthcheck(store: Arc<CassandraStore>, health: Arc<AtomicBool>) {
    tokio::spawn(async move {
        loop {
            let ok = store.healthcheck().await;
            health.store(ok, Ordering::Relaxed);
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    });
}

async fn shutdown_signal() {
    let _ = signal::ctrl_c().await;
    tracing::info!("shutdown signal received");
}
