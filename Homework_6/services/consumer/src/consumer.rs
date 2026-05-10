use crate::cassandra::{CassandraStore, ProcessOutcome};
use crate::dlq::DlqPublisher;
use crate::metrics::Metrics;
use crate::schema_registry::SchemaRegistryDecoder;
use anyhow::Context;
use futures::StreamExt;
use rdkafka::Message;
use rdkafka::consumer::{CommitMode, Consumer, StreamConsumer};
use rdkafka::message::BorrowedMessage;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

pub async fn run(
    consumer: Arc<StreamConsumer>,
    decoder: Arc<SchemaRegistryDecoder>,
    cassandra: Arc<CassandraStore>,
    dlq: Arc<DlqPublisher>,
    metrics: Arc<Metrics>,
    kafka_healthy: Arc<AtomicBool>,
) -> anyhow::Result<()> {
    let mut stream = consumer.stream();
    while let Some(message) = stream.next().await {
        match message {
            Ok(msg) => {
                kafka_healthy.store(true, Ordering::Relaxed);
                let timer = metrics.event_processing_duration_seconds.start_timer();
                if let Err(err) =
                    process_message(&consumer, &decoder, &cassandra, &dlq, &metrics, &msg).await
                {
                    tracing::error!(error = %err, "message processing failed");
                }
                timer.observe_duration();
            }
            Err(err) => {
                kafka_healthy.store(false, Ordering::Relaxed);
                tracing::error!(error = %err, "kafka stream error");
                tokio::time::sleep(Duration::from_millis(250)).await;
            }
        }
    }
    Ok(())
}

async fn process_message(
    consumer: &StreamConsumer,
    decoder: &SchemaRegistryDecoder,
    cassandra: &CassandraStore,
    dlq: &DlqPublisher,
    metrics: &Metrics,
    msg: &BorrowedMessage<'_>,
) -> anyhow::Result<()> {
    let payload = msg
        .payload()
        .ok_or_else(|| anyhow::anyhow!("empty kafka payload"))?;
    let decoded = decoder.decode(payload).await.context("decode failed")?;
    decoded
        .event
        .validate()
        .map_err(|e| anyhow::anyhow!(e.to_string()))?;

    let process_result = cassandra
        .process_event(&decoded.event, msg.partition(), msg.offset())
        .await;

    match process_result {
        Ok(outcome) => {
            tracing::info!(
                event_id = %decoded.event.event_id,
                event_type = decoded.event.event_type.as_str(),
                partition = msg.partition(),
                offset = msg.offset(),
                outcome = ?outcome,
                "event processed",
            );
            if matches!(
                outcome,
                ProcessOutcome::Applied | ProcessOutcome::Duplicate | ProcessOutcome::Stale
            ) {
                metrics
                    .events_processed_total
                    .with_label_values(&[decoded.event.event_type.as_str()])
                    .inc();
            }
            consumer.commit_message(msg, CommitMode::Sync)?;
            update_lag_metric(consumer, metrics, msg).await;
            Ok(())
        }
        Err(err) => {
            metrics.cassandra_write_errors_total.inc();
            let err_text = err.to_string();
            tracing::error!(
                event_id = %decoded.event.event_id,
                event_type = decoded.event.event_type.as_str(),
                partition = msg.partition(),
                offset = msg.offset(),
                error = %err_text,
                "event processing failed, sending to dlq",
            );
            dlq.publish(
                &decoded.event.event_id.to_string(),
                decoded.raw_json,
                err_text,
                "PROCESSING_ERROR".to_string(),
                msg.partition(),
                msg.offset(),
            )
            .await
            .map_err(|e| anyhow::anyhow!(e))
            .context("dlq publish failed")?;
            consumer.commit_message(msg, CommitMode::Sync)?;
            update_lag_metric(consumer, metrics, msg).await;
            Ok(())
        }
    }
}

async fn update_lag_metric(
    consumer: &StreamConsumer,
    metrics: &Metrics,
    msg: &BorrowedMessage<'_>,
) {
    if let Ok((_, high)) =
        consumer.fetch_watermarks(msg.topic(), msg.partition(), Duration::from_secs(1))
    {
        let lag = (high - (msg.offset() + 1)).max(0);
        metrics
            .consumer_lag
            .with_label_values(&[&msg.partition().to_string()])
            .set(lag as i64);
    }
}
