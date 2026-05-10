use chrono::Utc;
use rdkafka::producer::{FutureProducer, FutureRecord};
use rdkafka::util::Timeout;
use serde::Serialize;
use std::time::Duration;

#[derive(Clone)]
pub struct DlqPublisher {
    producer: FutureProducer,
    topic: String,
}

#[derive(Serialize)]
pub struct KafkaMetadata {
    pub partition: i32,
    pub offset: i64,
}

#[derive(Serialize)]
pub struct DlqMessage {
    pub original_event: serde_json::Value,
    pub error_reason: String,
    pub error_code: String,
    pub failed_at: String,
    pub kafka_metadata: KafkaMetadata,
}

impl DlqPublisher {
    pub fn new(producer: FutureProducer, topic: String) -> Self {
        Self { producer, topic }
    }

    pub async fn publish(
        &self,
        key: &str,
        original_event: serde_json::Value,
        error_reason: String,
        error_code: String,
        partition: i32,
        offset: i64,
    ) -> Result<(), String> {
        let payload = DlqMessage {
            original_event,
            error_reason,
            error_code,
            failed_at: Utc::now().to_rfc3339(),
            kafka_metadata: KafkaMetadata { partition, offset },
        };
        let body = serde_json::to_vec(&payload).map_err(|e| e.to_string())?;
        let record = FutureRecord::to(&self.topic).key(key).payload(&body);
        self.producer
            .send(record, Timeout::After(Duration::from_secs(10)))
            .await
            .map_err(|(e, _)| e.to_string())?;
        Ok(())
    }
}
