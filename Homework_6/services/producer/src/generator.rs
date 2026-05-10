use crate::config::Config;
use crate::publisher::Publisher;
use chrono::Utc;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::sync::Arc;
use tracing::warn;
use uuid::Uuid;
use warehouse_common::{EventType, OrderItem, WarehouseEventPayload};

pub async fn run(cfg: &Config, publisher: Arc<Publisher>) {
    let mut rng = StdRng::from_entropy();
    loop {
        tokio::time::sleep(cfg.generator_interval).await;
        let payload = random_event(&mut rng);
        if let Err(e) = publisher.publish(&payload).await {
            warn!(error = %e, "generator publish failed");
        }
    }
}

pub async fn generate_batch(
    publisher: &Arc<Publisher>,
    count: usize,
) -> Result<(usize, usize), String> {
    let mut rng = StdRng::from_entropy();
    let mut ok = 0usize;
    let mut failed = 0usize;

    for _ in 0..count {
        let payload = random_event(&mut rng);
        match publisher.publish(&payload).await {
            Ok(_) => ok += 1,
            Err(_) => failed += 1,
        }
    }

    if ok == 0 && failed > 0 {
        return Err("all generated events failed to publish".to_string());
    }

    Ok((ok, failed))
}

fn random_event(rng: &mut impl Rng) -> WarehouseEventPayload {
    let event_type = match rng.gen_range(0..8) {
        0 => EventType::ProductReceived,
        1 => EventType::ProductShipped,
        2 => EventType::ProductMoved,
        3 => EventType::ProductReserved,
        4 => EventType::ProductReleased,
        5 => EventType::InventoryCounted,
        6 => EventType::OrderCreated,
        _ => EventType::OrderCompleted,
    };
    let product_id = format!("SKU-{:03}", rng.gen_range(1..120));
    let zone_a = format!("ZONE-{}", (b'A' + rng.gen_range(0..6) as u8) as char);
    let zone_b = format!("ZONE-{}", (b'A' + rng.gen_range(0..6) as u8) as char);
    let quantity = rng.gen_range(1..70);
    let order_id = format!("ORDER-{:05}", rng.gen_range(1..99999));

    match event_type {
        EventType::ProductReceived => WarehouseEventPayload {
            event_id: Uuid::new_v4(),
            event_type,
            event_timestamp: Utc::now(),
            product_id: Some(product_id),
            zone_id: Some(zone_a),
            from_zone_id: None,
            to_zone_id: None,
            quantity: Some(quantity),
            counted_quantity: None,
            order_id: None,
            supplier_id: if rng.gen_bool(0.6) {
                Some(format!("SUP-{:03}", rng.gen_range(1..70)))
            } else {
                None
            },
            items: None,
        },
        EventType::ProductShipped => WarehouseEventPayload {
            event_id: Uuid::new_v4(),
            event_type,
            event_timestamp: Utc::now(),
            product_id: Some(product_id),
            zone_id: Some(zone_a),
            from_zone_id: None,
            to_zone_id: None,
            quantity: Some(quantity),
            counted_quantity: None,
            order_id: None,
            supplier_id: None,
            items: None,
        },
        EventType::ProductMoved => WarehouseEventPayload {
            event_id: Uuid::new_v4(),
            event_type,
            event_timestamp: Utc::now(),
            product_id: Some(product_id),
            zone_id: None,
            from_zone_id: Some(zone_a),
            to_zone_id: Some(zone_b),
            quantity: Some(quantity),
            counted_quantity: None,
            order_id: None,
            supplier_id: None,
            items: None,
        },
        EventType::ProductReserved => WarehouseEventPayload {
            event_id: Uuid::new_v4(),
            event_type,
            event_timestamp: Utc::now(),
            product_id: Some(product_id),
            zone_id: Some(zone_a),
            from_zone_id: None,
            to_zone_id: None,
            quantity: Some(quantity),
            counted_quantity: None,
            order_id: Some(order_id),
            supplier_id: None,
            items: None,
        },
        EventType::ProductReleased => WarehouseEventPayload {
            event_id: Uuid::new_v4(),
            event_type,
            event_timestamp: Utc::now(),
            product_id: Some(product_id),
            zone_id: Some(zone_a),
            from_zone_id: None,
            to_zone_id: None,
            quantity: Some(quantity),
            counted_quantity: None,
            order_id: Some(order_id),
            supplier_id: None,
            items: None,
        },
        EventType::InventoryCounted => WarehouseEventPayload {
            event_id: Uuid::new_v4(),
            event_type,
            event_timestamp: Utc::now(),
            product_id: Some(product_id),
            zone_id: Some(zone_a),
            from_zone_id: None,
            to_zone_id: None,
            quantity: None,
            counted_quantity: Some(rng.gen_range(0..120)),
            order_id: None,
            supplier_id: None,
            items: None,
        },
        EventType::OrderCreated | EventType::OrderCompleted => {
            let item_count = rng.gen_range(1..4);
            let items = (0..item_count)
                .map(|_| OrderItem {
                    product_id: format!("SKU-{:03}", rng.gen_range(1..120)),
                    zone_id: format!("ZONE-{}", (b'A' + rng.gen_range(0..6) as u8) as char),
                    quantity: rng.gen_range(1..25),
                })
                .collect::<Vec<_>>();
            WarehouseEventPayload {
                event_id: Uuid::new_v4(),
                event_type,
                event_timestamp: Utc::now(),
                product_id: None,
                zone_id: None,
                from_zone_id: None,
                to_zone_id: None,
                quantity: None,
                counted_quantity: None,
                order_id: Some(order_id),
                supplier_id: None,
                items: Some(items),
            }
        }
    }
}
