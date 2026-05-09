use crate::event::MovieEventPayload;
use apache_avro::Schema;
use rdkafka::config::ClientConfig;
use rdkafka::producer::{FutureProducer, FutureRecord};
use rdkafka::util::Timeout;
use serde::Deserialize;
use std::sync::Arc;
use std::time::Duration;
use tracing::{error, warn};

#[derive(Deserialize)]
struct SubjectVersion {
    id: i32,
    #[allow(dead_code)]
    version: i32,
    schema: String,
}

pub struct Publisher {
    producer: FutureProducer,
    topic: String,
    schema: Arc<Schema>,
    schema_id: i32,
}

impl Publisher {
    pub async fn connect(
        kafka_bootstrap: &str,
        topic: String,
        schema_registry_url: &str,
        subject: &str,
    ) -> Result<Arc<Self>, String> {
        let http = reqwest::Client::builder()
            .build()
            .map_err(|e| e.to_string())?;

        let (schema_id, schema) =
            Self::fetch_subject_with_retry(&http, schema_registry_url, subject).await?;

        let producer: FutureProducer = ClientConfig::new()
            .set("bootstrap.servers", kafka_bootstrap)
            .set("acks", "all")
            .set("enable.idempotence", "true")
            .set("message.timeout.ms", "120000")
            .set("retries", "10")
            .set("retry.backoff.ms", "200")
            .create()
            .map_err(|e| e.to_string())?;

        Ok(Arc::new(Publisher {
            producer,
            topic,
            schema,
            schema_id,
        }))
    }

    async fn fetch_subject_with_retry(
        http: &reqwest::Client,
        base: &str,
        subject: &str,
    ) -> Result<(i32, Arc<Schema>), String> {
        let url = format!(
            "{}/subjects/{}/versions/latest",
            base.trim_end_matches('/'),
            subject
        );
        let mut delay = Duration::from_millis(500);
        for attempt in 1..=30u32 {
            match http.get(&url).send().await {
                Ok(resp) if resp.status().is_success() => {
                    let body: SubjectVersion = resp.json().await.map_err(|e| e.to_string())?;
                    let remote = Schema::parse_str(&body.schema).map_err(|e| e.to_string())?;
                    return Ok((body.id, Arc::new(remote)));
                }
                Ok(resp) => {
                    warn!(
                        attempt,
                        status = %resp.status(),
                        "schema registry GET latest failed, retrying"
                    );
                }
                Err(e) => {
                    warn!(attempt, error = %e, "schema registry request error, retrying");
                }
            }
            tokio::time::sleep(delay).await;
            delay = (delay * 2).min(Duration::from_secs(30));
        }
        Err("could not fetch latest schema from Schema Registry".into())
    }

    fn confluent_payload(schema_id: i32, avro_body: &[u8]) -> Vec<u8> {
        let mut out = Vec::with_capacity(1 + 4 + avro_body.len());
        out.push(0u8);
        out.extend_from_slice(&schema_id.to_be_bytes());
        out.extend_from_slice(avro_body);
        out
    }

    pub async fn publish(&self, event: &MovieEventPayload) -> Result<(), String> {
        let avro_bytes = event
            .encode_avro(&self.schema)
            .map_err(|e| e.to_string())?;
        let payload = Self::confluent_payload(self.schema_id, &avro_bytes);
        let key = event.user_id.as_bytes();

        let mut backoff = Duration::from_millis(100);
        for attempt in 1..=8u32 {
            let record = FutureRecord::to(&self.topic)
                .key(key)
                .payload(&payload);
            match self
                .producer
                .send(record, Timeout::After(Duration::from_secs(60)))
                .await
            {
                Ok((partition, offset)) => {
                    tracing::info!(
                        event_id = %event.event_id,
                        event_type = ?event.event_type,
                        timestamp = %event.timestamp.to_rfc3339(),
                        partition,
                        offset,
                        "published movie event"
                    );
                    return Ok(());
                }
                Err((e, _)) => {
                    error!(attempt, error = %e, "kafka publish failed");
                    if attempt == 8 {
                        return Err(e.to_string());
                    }
                    tokio::time::sleep(backoff).await;
                    backoff = (backoff * 2).min(Duration::from_secs(10));
                }
            }
        }
        Err("kafka publish failed after retries".into())
    }
}
