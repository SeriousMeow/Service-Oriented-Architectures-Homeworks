use anyhow::Context;
use apache_avro::schema::Name;
use apache_avro::types::Value as AvroValue;
use chrono::{DateTime, TimeZone, Utc};
use schema_registry_client::rest::client_config::ClientConfig as SrClientConfig;
use schema_registry_client::rest::schema_registry_client::{Client, SchemaRegistryClient};
use schema_registry_client::serdes::avro::AvroDeserializer;
use schema_registry_client::serdes::config::DeserializerConfig;
use schema_registry_client::serdes::serde::{
    SerdeFormat, SerdeType, SerializationContext, SubjectNameStrategyType,
};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;
use warehouse_common::{EventType, OrderItem, WarehouseEventPayload};

pub struct DecodedEvent {
    pub event: WarehouseEventPayload,
    pub raw_json: serde_json::Value,
}

pub struct SchemaRegistryDecoder {
    client: Arc<SchemaRegistryClient>,
    topic: String,
    deser_conf: DeserializerConfig,
}

impl SchemaRegistryDecoder {
    pub fn new(base_url: String, topic: String) -> Self {
        let mut sr_cfg = SrClientConfig::new(vec![base_url.trim_end_matches('/').to_string()]);
        sr_cfg.max_retries = 5;
        sr_cfg.retries_wait_ms = 400;
        sr_cfg.retries_max_wait_ms = 20_000;
        let client: SchemaRegistryClient = Client::new(sr_cfg);
        let mut deser_conf = DeserializerConfig::new(None, false, HashMap::new());
        deser_conf.subject_name_strategy_type = SubjectNameStrategyType::Record;
        Self {
            client: Arc::new(client),
            topic,
            deser_conf,
        }
    }

    pub async fn decode(&self, payload: &[u8]) -> anyhow::Result<DecodedEvent> {
        let deser = AvroDeserializer::new(&*self.client, None, self.deser_conf.clone())
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        let ctx = SerializationContext {
            topic: self.topic.clone(),
            serde_type: SerdeType::Value,
            serde_format: SerdeFormat::Avro,
            headers: None,
        };
        let named = deser
            .deserialize(&ctx, payload)
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        let event = decode_event(named.name.as_ref(), &named.value)
            .context("avro->event conversion failed")?;
        let raw_json = serde_json::to_value(&event).context("event json conversion failed")?;
        Ok(DecodedEvent { event, raw_json })
    }
}

fn decode_event(
    record_name: Option<&Name>,
    value: &AvroValue,
) -> anyhow::Result<WarehouseEventPayload> {
    let fields = record_fields(value)?;
    let event_id = parse_uuid(required_field(fields, "event_id")?)?;
    let event_timestamp = parse_timestamp(required_field(fields, "event_timestamp")?)?;
    let event_type = match parse_optional_field(fields, "event_type", parse_optional_string)? {
        Some(raw_type) => parse_event_type(&raw_type)?,
        None => infer_type_from_record_name(record_name)?,
    };
    Ok(WarehouseEventPayload {
        event_id,
        event_type,
        event_timestamp,
        product_id: parse_optional_field(fields, "product_id", parse_optional_string)?,
        zone_id: parse_optional_field(fields, "zone_id", parse_optional_string)?,
        from_zone_id: parse_optional_field(fields, "from_zone_id", parse_optional_string)?,
        to_zone_id: parse_optional_field(fields, "to_zone_id", parse_optional_string)?,
        quantity: parse_optional_field(fields, "quantity", parse_optional_i32)?,
        counted_quantity: parse_optional_field(fields, "counted_quantity", parse_optional_i32)?,
        order_id: parse_optional_field(fields, "order_id", parse_optional_string)?,
        supplier_id: parse_optional_field(fields, "supplier_id", parse_optional_string)?,
        items: parse_optional_field(fields, "items", parse_optional_items)?,
    })
}

fn record_fields(value: &AvroValue) -> anyhow::Result<&[(String, AvroValue)]> {
    match value {
        AvroValue::Record(fields) => Ok(fields.as_slice()),
        _ => anyhow::bail!("expected avro record"),
    }
}

fn required_field<'a>(
    fields: &'a [(String, AvroValue)],
    name: &str,
) -> anyhow::Result<&'a AvroValue> {
    optional_field(fields, name).ok_or_else(|| anyhow::anyhow!("missing field {name}"))
}

fn optional_field<'a>(fields: &'a [(String, AvroValue)], name: &str) -> Option<&'a AvroValue> {
    fields.iter().find_map(|(field_name, value)| {
        if field_name == name {
            Some(value)
        } else {
            None
        }
    })
}

fn parse_optional_field<T>(
    fields: &[(String, AvroValue)],
    name: &str,
    parser: fn(&AvroValue) -> anyhow::Result<Option<T>>,
) -> anyhow::Result<Option<T>> {
    match optional_field(fields, name) {
        Some(value) => parser(value),
        None => Ok(None),
    }
}

fn unwrapped_value(value: &AvroValue) -> &AvroValue {
    if let AvroValue::Union(_, inner) = value {
        inner
    } else {
        value
    }
}

fn parse_uuid(value: &AvroValue) -> anyhow::Result<Uuid> {
    let text = parse_string(value)?;
    Uuid::parse_str(&text).context("invalid event_id")
}

fn parse_string(value: &AvroValue) -> anyhow::Result<String> {
    match unwrapped_value(value) {
        AvroValue::String(text) => Ok(text.clone()),
        _ => anyhow::bail!("expected string value"),
    }
}

fn parse_optional_string(value: &AvroValue) -> anyhow::Result<Option<String>> {
    match unwrapped_value(value) {
        AvroValue::Null => Ok(None),
        AvroValue::String(text) => Ok(Some(text.clone())),
        _ => anyhow::bail!("expected optional string value"),
    }
}

fn parse_optional_i32(value: &AvroValue) -> anyhow::Result<Option<i32>> {
    match unwrapped_value(value) {
        AvroValue::Null => Ok(None),
        AvroValue::Int(v) => Ok(Some(*v)),
        AvroValue::Long(v) => Ok(Some(
            i32::try_from(*v).context("numeric value out of i32 range")?,
        )),
        _ => anyhow::bail!("expected optional int value"),
    }
}

fn parse_optional_items(value: &AvroValue) -> anyhow::Result<Option<Vec<OrderItem>>> {
    match unwrapped_value(value) {
        AvroValue::Null => Ok(None),
        AvroValue::Array(items) => {
            let mut parsed = Vec::with_capacity(items.len());
            for item in items {
                let item_fields = record_fields(item)?;
                let product_id = parse_string(required_field(item_fields, "product_id")?)?;
                let zone_id = parse_string(required_field(item_fields, "zone_id")?)?;
                let quantity = match unwrapped_value(required_field(item_fields, "quantity")?) {
                    AvroValue::Int(v) => *v,
                    AvroValue::Long(v) => {
                        i32::try_from(*v).context("order item quantity out of i32 range")?
                    }
                    _ => anyhow::bail!("expected order item quantity int"),
                };
                parsed.push(OrderItem {
                    product_id,
                    zone_id,
                    quantity,
                });
            }
            Ok(Some(parsed))
        }
        _ => anyhow::bail!("expected optional array value"),
    }
}

fn parse_timestamp(value: &AvroValue) -> anyhow::Result<DateTime<Utc>> {
    let millis = match unwrapped_value(value) {
        AvroValue::TimestampMillis(v) => *v,
        AvroValue::Long(v) => *v,
        AvroValue::Int(v) => i64::from(*v),
        AvroValue::String(raw) => parse_timestamp_string(raw)?,
        AvroValue::TimestampMicros(v) => *v / 1000,
        _ => anyhow::bail!("unsupported timestamp value"),
    };
    Utc.timestamp_millis_opt(millis)
        .single()
        .ok_or_else(|| anyhow::anyhow!("timestamp out of range"))
}

fn parse_timestamp_string(raw: &str) -> anyhow::Result<i64> {
    if let Ok(parsed) = DateTime::parse_from_rfc3339(raw) {
        return Ok(parsed.with_timezone(&Utc).timestamp_millis());
    }
    raw.parse::<i64>()
        .context("timestamp string is not RFC3339 or millis")
}

fn parse_event_type(raw: &str) -> anyhow::Result<EventType> {
    match raw {
        "PRODUCT_RECEIVED" | "ProductReceived" => Ok(EventType::ProductReceived),
        "PRODUCT_SHIPPED" | "ProductShipped" => Ok(EventType::ProductShipped),
        "PRODUCT_MOVED" | "ProductMoved" => Ok(EventType::ProductMoved),
        "PRODUCT_RESERVED" | "ProductReserved" => Ok(EventType::ProductReserved),
        "PRODUCT_RELEASED" | "ProductReleased" => Ok(EventType::ProductReleased),
        "INVENTORY_COUNTED" | "InventoryCounted" => Ok(EventType::InventoryCounted),
        "ORDER_CREATED" | "OrderCreated" => Ok(EventType::OrderCreated),
        "ORDER_COMPLETED" | "OrderCompleted" => Ok(EventType::OrderCompleted),
        _ => anyhow::bail!("unknown event_type value"),
    }
}

fn infer_type_from_record_name(name: Option<&Name>) -> anyhow::Result<EventType> {
    let n = name
        .ok_or_else(|| anyhow::anyhow!("event_type is missing and record name is absent"))?
        .name
        .as_str();
    match n {
        "ProductReceived" | "ProductReceivedV1" | "ProductReceivedV2" => {
            Ok(EventType::ProductReceived)
        }
        "ProductShipped" => Ok(EventType::ProductShipped),
        "ProductMoved" => Ok(EventType::ProductMoved),
        "ProductReserved" => Ok(EventType::ProductReserved),
        "ProductReleased" => Ok(EventType::ProductReleased),
        "InventoryCounted" => Ok(EventType::InventoryCounted),
        "OrderCreated" => Ok(EventType::OrderCreated),
        "OrderCompleted" => Ok(EventType::OrderCompleted),
        _ => anyhow::bail!("event_type is missing and cannot be inferred"),
    }
}
