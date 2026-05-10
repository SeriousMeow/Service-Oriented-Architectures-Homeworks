use crate::config::Config;
use crate::event::WarehouseEventKafkaExt;
use crate::schema::{EmbeddedAvroSchema, schema_for, schema_registry_subject};
use rdkafka::config::ClientConfig;
use rdkafka::producer::{FutureProducer, FutureRecord};
use rdkafka::util::Timeout;
use schema_registry_client::rest::client_config::ClientConfig as SrClientConfig;
use schema_registry_client::rest::models::Schema as SrSchema;
use schema_registry_client::rest::schema_registry_client::{Client, SchemaRegistryClient};
use schema_registry_client::serdes::avro::AvroSerializer;
use schema_registry_client::serdes::config::SerializerConfig;
use schema_registry_client::serdes::serde::{
    SerdeFormat, SerdeType, SerializationContext, SubjectNameStrategyType,
};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tracing::error;
use warehouse_common::{EventType, WarehouseEventPayload};

pub struct Publisher {
    producer: FutureProducer,
    topic: String,
    sr: Arc<SchemaRegistryClient>,
    ser_conf: SerializerConfig,
    bindings: HashMap<EventType, SrSchema>,
    product_received_v1: SrSchema,
    product_received_v2: SrSchema,
}

impl Publisher {
    pub async fn connect(cfg: &Config) -> Result<Arc<Self>, String> {
        let mut sr_cfg = SrClientConfig::new(vec![
            cfg.schema_registry_url.trim_end_matches('/').to_string(),
        ]);
        sr_cfg.max_retries = 5;
        sr_cfg.retries_wait_ms = 400;
        sr_cfg.retries_max_wait_ms = 20_000;
        let sr: SchemaRegistryClient = Client::new(sr_cfg);
        let sr = Arc::new(sr);

        Self::register_schema(sr.as_ref(), EmbeddedAvroSchema::ProductReceivedV1.as_str()).await?;
        Self::register_schema(sr.as_ref(), EmbeddedAvroSchema::ProductReceivedV2.as_str()).await?;

        let mut bindings = HashMap::new();
        for event_type in [
            EventType::ProductShipped,
            EventType::ProductMoved,
            EventType::ProductReserved,
            EventType::ProductReleased,
            EventType::InventoryCounted,
            EventType::OrderCreated,
            EventType::OrderCompleted,
        ] {
            let schema_str = schema_for(event_type);
            Self::register_schema(sr.as_ref(), schema_str).await?;
            bindings.insert(event_type, Self::sr_schema(schema_str));
        }

        let mut ser_conf = SerializerConfig::new(true, None, false, false, HashMap::new());
        ser_conf.subject_name_strategy_type = SubjectNameStrategyType::Record;

        let producer: FutureProducer = ClientConfig::new()
            .set("bootstrap.servers", &cfg.kafka_bootstrap)
            .set("acks", "all")
            .set("enable.idempotence", "true")
            .set("message.timeout.ms", "120000")
            .set("retries", "10")
            .set("retry.backoff.ms", "200")
            .create()
            .map_err(|e| e.to_string())?;

        Ok(Arc::new(Publisher {
            producer,
            topic: cfg.warehouse_events_topic.clone(),
            sr,
            ser_conf,
            bindings,
            product_received_v1: Self::sr_schema(EmbeddedAvroSchema::ProductReceivedV1.as_str()),
            product_received_v2: Self::sr_schema(EmbeddedAvroSchema::ProductReceivedV2.as_str()),
        }))
    }

    fn sr_schema(schema_str: &str) -> SrSchema {
        SrSchema {
            schema_type: Some("AVRO".to_string()),
            references: None,
            metadata: None,
            rule_set: None,
            schema: schema_str.to_string(),
        }
    }

    async fn register_schema(
        client: &SchemaRegistryClient,
        schema_str: &str,
    ) -> Result<(), String> {
        let subject = schema_registry_subject(schema_str)?;
        let sr_body = Self::sr_schema(schema_str);
        client
            .register_schema(&subject, &sr_body, false)
            .await
            .map_err(|e| format!("schema registry register failed for subject {subject}: {e}"))?;
        Ok(())
    }

    pub async fn publish(&self, event: &WarehouseEventPayload) -> Result<(), String> {
        let (sr_schema_ref, avro_val) = if event.event_type == EventType::ProductReceived {
            let use_v2 = event.supplier_id.is_some();
            let schema = if use_v2 {
                &self.product_received_v2
            } else {
                &self.product_received_v1
            };
            (schema, event.avro_value_for_kafka())
        } else {
            let schema = self
                .bindings
                .get(&event.event_type)
                .ok_or_else(|| "missing schema binding".to_string())?;
            (schema, event.avro_value_for_kafka())
        };

        let ser = AvroSerializer::new(&*self.sr, Some(sr_schema_ref), None, self.ser_conf.clone())
            .map_err(|e| e.to_string())?;
        let ctx = SerializationContext {
            topic: self.topic.clone(),
            serde_type: SerdeType::Value,
            serde_format: SerdeFormat::Avro,
            headers: None,
        };
        let payload = ser
            .serialize(&ctx, avro_val)
            .await
            .map_err(|e| e.to_string())?;

        let key = event.partition_key();
        let record = FutureRecord::to(&self.topic).key(&key).payload(&payload);
        match self
            .producer
            .send(record, Timeout::After(Duration::from_secs(60)))
            .await
        {
            Ok((partition, offset)) => {
                tracing::info!(
                    event_id = %event.event_id,
                    event_type = event.event_type.as_str(),
                    timestamp = %event.event_timestamp.to_rfc3339(),
                    partition,
                    offset,
                    "published warehouse event"
                );
                Ok(())
            }
            Err((e, _)) => {
                error!(error = %e, "kafka publish failed");
                Err(e.to_string())
            }
        }
    }
}
