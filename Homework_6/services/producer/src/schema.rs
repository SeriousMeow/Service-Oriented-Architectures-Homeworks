use apache_avro::Schema;
use warehouse_common::EventType;

#[derive(Clone, Copy)]
pub enum EmbeddedAvroSchema {
    ProductReceivedV1,
    ProductReceivedV2,
    ProductShipped,
    ProductMoved,
    ProductReserved,
    ProductReleased,
    InventoryCounted,
    OrderCreated,
    OrderCompleted,
}

impl EmbeddedAvroSchema {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ProductReceivedV1 => {
                include_str!("../../../schemas/avro/ProductReceivedV1.avsc")
            }
            Self::ProductReceivedV2 => {
                include_str!("../../../schemas/avro/ProductReceivedV2.avsc")
            }
            Self::ProductShipped => include_str!("../../../schemas/avro/ProductShipped.avsc"),
            Self::ProductMoved => include_str!("../../../schemas/avro/ProductMoved.avsc"),
            Self::ProductReserved => include_str!("../../../schemas/avro/ProductReserved.avsc"),
            Self::ProductReleased => include_str!("../../../schemas/avro/ProductReleased.avsc"),
            Self::InventoryCounted => include_str!("../../../schemas/avro/InventoryCounted.avsc"),
            Self::OrderCreated => include_str!("../../../schemas/avro/OrderCreated.avsc"),
            Self::OrderCompleted => include_str!("../../../schemas/avro/OrderCompleted.avsc"),
        }
    }
}

pub fn schema_for(event_type: EventType) -> &'static str {
    embedded_schema_for(event_type).as_str()
}

fn embedded_schema_for(event_type: EventType) -> EmbeddedAvroSchema {
    match event_type {
        EventType::ProductReceived => EmbeddedAvroSchema::ProductReceivedV2,
        EventType::ProductShipped => EmbeddedAvroSchema::ProductShipped,
        EventType::ProductMoved => EmbeddedAvroSchema::ProductMoved,
        EventType::ProductReserved => EmbeddedAvroSchema::ProductReserved,
        EventType::ProductReleased => EmbeddedAvroSchema::ProductReleased,
        EventType::InventoryCounted => EmbeddedAvroSchema::InventoryCounted,
        EventType::OrderCreated => EmbeddedAvroSchema::OrderCreated,
        EventType::OrderCompleted => EmbeddedAvroSchema::OrderCompleted,
    }
}

pub fn schema_registry_subject(schema_json: &str) -> Result<String, String> {
    let s = Schema::parse_str(schema_json).map_err(|e| e.to_string())?;
    match s {
        Schema::Record(r) => Ok(match &r.name.namespace {
            Some(ns) => format!("{ns}.{}", r.name.name),
            None => r.name.name.clone(),
        }),
        _ => Err("expected Avro record schema".to_string()),
    }
}
