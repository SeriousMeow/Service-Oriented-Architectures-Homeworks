use crate::warehouse::events::{
    InventoryCounted, OrderCompleted, OrderCompletedItem, OrderCreated, OrderItem as AvroOrderItem,
    ProductMoved, ProductReceivedV1, ProductReceivedV2, ProductReleased, ProductReserved,
    ProductShipped,
};
use apache_avro::types::Value as AvroValue;
use warehouse_common::event::{EventType, WarehouseEventPayload};

pub trait WarehouseEventKafkaExt {
    fn avro_value_for_kafka(&self) -> AvroValue;
    fn partition_key(&self) -> String;
}

impl WarehouseEventKafkaExt for WarehouseEventPayload {
    fn avro_value_for_kafka(&self) -> AvroValue {
        let ts = self.event_timestamp.naive_utc();
        let value = match self.event_type {
            EventType::ProductReceived => {
                if self.supplier_id.is_some() {
                    apache_avro::to_value(&ProductReceivedV2 {
                        event_id: self.event_id.to_string(),
                        product_id: self.product_id.clone().unwrap_or_default(),
                        zone_id: self.zone_id.clone().unwrap_or_default(),
                        quantity: self.quantity.unwrap_or_default(),
                        event_timestamp: ts,
                        supplier_id: self.supplier_id.clone(),
                    })
                } else {
                    apache_avro::to_value(&ProductReceivedV1 {
                        event_id: self.event_id.to_string(),
                        product_id: self.product_id.clone().unwrap_or_default(),
                        zone_id: self.zone_id.clone().unwrap_or_default(),
                        quantity: self.quantity.unwrap_or_default(),
                        event_timestamp: ts,
                    })
                }
            }
            EventType::ProductShipped => apache_avro::to_value(&ProductShipped {
                event_id: self.event_id.to_string(),
                event_type: self.event_type.as_str().to_string(),
                event_timestamp: ts,
                product_id: self.product_id.clone().unwrap_or_default(),
                zone_id: self.zone_id.clone().unwrap_or_default(),
                quantity: self.quantity.unwrap_or_default(),
            }),
            EventType::ProductMoved => apache_avro::to_value(&ProductMoved {
                event_id: self.event_id.to_string(),
                event_type: self.event_type.as_str().to_string(),
                event_timestamp: ts,
                product_id: self.product_id.clone().unwrap_or_default(),
                from_zone_id: self.from_zone_id.clone().unwrap_or_default(),
                to_zone_id: self.to_zone_id.clone().unwrap_or_default(),
                quantity: self.quantity.unwrap_or_default(),
            }),
            EventType::ProductReserved => apache_avro::to_value(&ProductReserved {
                event_id: self.event_id.to_string(),
                event_type: self.event_type.as_str().to_string(),
                event_timestamp: ts,
                product_id: self.product_id.clone().unwrap_or_default(),
                zone_id: self.zone_id.clone().unwrap_or_default(),
                quantity: self.quantity.unwrap_or_default(),
                order_id: self.order_id.clone(),
            }),
            EventType::ProductReleased => apache_avro::to_value(&ProductReleased {
                event_id: self.event_id.to_string(),
                event_type: self.event_type.as_str().to_string(),
                event_timestamp: ts,
                product_id: self.product_id.clone().unwrap_or_default(),
                zone_id: self.zone_id.clone().unwrap_or_default(),
                quantity: self.quantity.unwrap_or_default(),
                order_id: self.order_id.clone(),
            }),
            EventType::InventoryCounted => apache_avro::to_value(&InventoryCounted {
                event_id: self.event_id.to_string(),
                event_type: self.event_type.as_str().to_string(),
                event_timestamp: ts,
                product_id: self.product_id.clone().unwrap_or_default(),
                zone_id: self.zone_id.clone().unwrap_or_default(),
                counted_quantity: self.counted_quantity.unwrap_or_default(),
            }),
            EventType::OrderCreated => apache_avro::to_value(&OrderCreated {
                event_id: self.event_id.to_string(),
                event_type: self.event_type.as_str().to_string(),
                event_timestamp: ts,
                order_id: self.order_id.clone().unwrap_or_default(),
                items: self
                    .items
                    .as_ref()
                    .cloned()
                    .unwrap_or_default()
                    .into_iter()
                    .map(|it| AvroOrderItem {
                        product_id: it.product_id,
                        zone_id: it.zone_id,
                        quantity: it.quantity,
                    })
                    .collect(),
            }),
            EventType::OrderCompleted => apache_avro::to_value(&OrderCompleted {
                event_id: self.event_id.to_string(),
                event_type: self.event_type.as_str().to_string(),
                event_timestamp: ts,
                order_id: self.order_id.clone().unwrap_or_default(),
                items: self
                    .items
                    .as_ref()
                    .cloned()
                    .unwrap_or_default()
                    .into_iter()
                    .map(|it| OrderCompletedItem {
                        product_id: it.product_id,
                        zone_id: it.zone_id,
                        quantity: it.quantity,
                    })
                    .collect(),
            }),
        };
        value.expect("avro value from generated event struct")
    }

    fn partition_key(&self) -> String {
        if let Some(product_id) = &self.product_id {
            return product_id.clone();
        }
        if let Some(order_id) = &self.order_id {
            return order_id.clone();
        }
        self.event_id.to_string()
    }
}
