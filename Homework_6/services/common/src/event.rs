use chrono::{DateTime, TimeZone, Utc};
use serde::{Deserialize, Deserializer, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EventType {
    ProductReceived,
    ProductShipped,
    ProductMoved,
    ProductReserved,
    ProductReleased,
    InventoryCounted,
    OrderCreated,
    OrderCompleted,
}

impl EventType {
    pub fn as_str(self) -> &'static str {
        match self {
            EventType::ProductReceived => "PRODUCT_RECEIVED",
            EventType::ProductShipped => "PRODUCT_SHIPPED",
            EventType::ProductMoved => "PRODUCT_MOVED",
            EventType::ProductReserved => "PRODUCT_RESERVED",
            EventType::ProductReleased => "PRODUCT_RELEASED",
            EventType::InventoryCounted => "INVENTORY_COUNTED",
            EventType::OrderCreated => "ORDER_CREATED",
            EventType::OrderCompleted => "ORDER_COMPLETED",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderItem {
    pub product_id: String,
    pub zone_id: String,
    pub quantity: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WarehouseEventPayload {
    pub event_id: Uuid,
    pub event_type: EventType,
    #[serde(deserialize_with = "deserialize_timestamp")]
    pub event_timestamp: DateTime<Utc>,
    pub product_id: Option<String>,
    pub zone_id: Option<String>,
    pub from_zone_id: Option<String>,
    pub to_zone_id: Option<String>,
    pub quantity: Option<i32>,
    pub counted_quantity: Option<i32>,
    pub order_id: Option<String>,
    pub supplier_id: Option<String>,
    pub items: Option<Vec<OrderItem>>,
}

#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    #[error("product_id must be non-empty")]
    ProductId,
    #[error("zone_id must be non-empty")]
    ZoneId,
    #[error("from_zone_id must be non-empty")]
    FromZoneId,
    #[error("to_zone_id must be non-empty")]
    ToZoneId,
    #[error("quantity must be positive")]
    Quantity,
    #[error("counted_quantity must be >= 0")]
    CountedQuantity,
    #[error("order_id must be non-empty")]
    OrderId,
    #[error("items must be a non-empty array")]
    Items,
    #[error("items contain invalid values")]
    InvalidItems,
}

impl WarehouseEventPayload {
    pub fn validate(&self) -> Result<(), ValidationError> {
        match self.event_type {
            EventType::ProductReceived => {
                required_non_empty(&self.product_id).map_err(|_| ValidationError::ProductId)?;
                required_non_empty(&self.zone_id).map_err(|_| ValidationError::ZoneId)?;
                positive(self.quantity).map_err(|_| ValidationError::Quantity)?;
            }
            EventType::ProductShipped => {
                required_non_empty(&self.product_id).map_err(|_| ValidationError::ProductId)?;
                required_non_empty(&self.zone_id).map_err(|_| ValidationError::ZoneId)?;
                positive(self.quantity).map_err(|_| ValidationError::Quantity)?;
            }
            EventType::ProductMoved => {
                required_non_empty(&self.product_id).map_err(|_| ValidationError::ProductId)?;
                required_non_empty(&self.from_zone_id).map_err(|_| ValidationError::FromZoneId)?;
                required_non_empty(&self.to_zone_id).map_err(|_| ValidationError::ToZoneId)?;
                positive(self.quantity).map_err(|_| ValidationError::Quantity)?;
            }
            EventType::ProductReserved | EventType::ProductReleased => {
                required_non_empty(&self.product_id).map_err(|_| ValidationError::ProductId)?;
                required_non_empty(&self.zone_id).map_err(|_| ValidationError::ZoneId)?;
                positive(self.quantity).map_err(|_| ValidationError::Quantity)?;
            }
            EventType::InventoryCounted => {
                required_non_empty(&self.product_id).map_err(|_| ValidationError::ProductId)?;
                required_non_empty(&self.zone_id).map_err(|_| ValidationError::ZoneId)?;
                non_negative(self.counted_quantity)
                    .map_err(|_| ValidationError::CountedQuantity)?;
            }
            EventType::OrderCreated | EventType::OrderCompleted => {
                required_non_empty(&self.order_id).map_err(|_| ValidationError::OrderId)?;
                let items = self.items.as_ref().ok_or(ValidationError::Items)?;
                if items.is_empty() {
                    return Err(ValidationError::Items);
                }
                if items.iter().any(|it| {
                    it.product_id.trim().is_empty()
                        || it.zone_id.trim().is_empty()
                        || it.quantity <= 0
                }) {
                    return Err(ValidationError::InvalidItems);
                }
            }
        }
        Ok(())
    }
}

fn required_non_empty(value: &Option<String>) -> Result<(), ()> {
    match value {
        Some(v) if !v.trim().is_empty() => Ok(()),
        _ => Err(()),
    }
}

fn positive(value: Option<i32>) -> Result<(), ()> {
    match value {
        Some(v) if v > 0 => Ok(()),
        _ => Err(()),
    }
}

fn non_negative(value: Option<i32>) -> Result<(), ()> {
    match value {
        Some(v) if v >= 0 => Ok(()),
        _ => Err(()),
    }
}

fn deserialize_timestamp<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Ts {
        Str(String),
        Millis(i64),
    }
    match Ts::deserialize(deserializer)? {
        Ts::Str(s) => {
            if let Ok(dt) = DateTime::parse_from_rfc3339(&s) {
                return Ok(dt.with_timezone(&Utc));
            }
            let ms: i64 = s.parse().map_err(serde::de::Error::custom)?;
            Utc.timestamp_millis_opt(ms)
                .single()
                .ok_or_else(|| serde::de::Error::custom("timestamp millis out of range"))
        }
        Ts::Millis(ms) => Utc
            .timestamp_millis_opt(ms)
            .single()
            .ok_or_else(|| serde::de::Error::custom("timestamp millis out of range")),
    }
}
