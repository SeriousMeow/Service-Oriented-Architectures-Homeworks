use anyhow::Context;
use chrono::Utc;
use scylla::client::session::Session;
use scylla::client::session_builder::SessionBuilder;
use scylla::statement::Consistency;
use scylla::statement::batch::{Batch, BatchType};
use scylla::statement::unprepared::Statement;
use scylla::value::CqlValue;
use std::collections::HashMap;
use std::sync::Arc;
use warehouse_common::{EventType, OrderItem, WarehouseEventPayload};

#[derive(Clone)]
pub struct CassandraStore {
    session: Arc<Session>,
    read_consistency: Consistency,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessOutcome {
    Applied,
    Duplicate,
    Stale,
}

#[derive(Default, Clone, Copy)]
struct ZoneState {
    available: i32,
    reserved: i32,
}

#[derive(Default, Clone, Copy)]
struct ProductTotals {
    available: i32,
    reserved: i32,
}

impl CassandraStore {
    pub async fn connect(nodes: &[String], read_consistency: Consistency) -> anyhow::Result<Self> {
        let mut builder = SessionBuilder::new();
        for node in nodes {
            builder = builder.known_node(node);
        }
        let session = builder.build().await.context("cassandra connect failed")?;
        Ok(Self {
            session: Arc::new(session),
            read_consistency,
        })
    }

    pub async fn healthcheck(&self) -> bool {
        self.session
            .query_unpaged(include_str!("../queries/healthcheck.cql").trim(), ())
            .await
            .is_ok()
    }

    pub async fn process_event(
        &self,
        event: &WarehouseEventPayload,
        kafka_partition: i32,
        kafka_offset: i64,
    ) -> anyhow::Result<ProcessOutcome> {
        if self.is_duplicate(&event.event_id.to_string()).await? {
            return Ok(ProcessOutcome::Duplicate);
        }

        let touched = touched_entities(event)?;
        for (product_id, zone_id) in &touched {
            if self
                .is_stale(
                    product_id,
                    zone_id,
                    event.event_timestamp.timestamp_millis(),
                )
                .await?
            {
                self.mark_processed(event, kafka_partition, kafka_offset)
                    .await?;
                return Ok(ProcessOutcome::Stale);
            }
        }

        let mut zone_states = HashMap::new();
        for (product_id, zone_id) in &touched {
            let state = self.read_zone_state(product_id, zone_id).await?;
            zone_states.insert((product_id.clone(), zone_id.clone()), state);
        }

        let mut product_deltas: HashMap<String, (i32, i32)> = HashMap::new();
        match event.event_type {
            EventType::ProductReceived => {
                let product_id = req(&event.product_id, "product_id")?;
                let zone_id = req(&event.zone_id, "zone_id")?;
                let quantity = req_i32(event.quantity, "quantity")?;
                add_delta(
                    &mut zone_states,
                    &product_id,
                    &zone_id,
                    quantity,
                    0,
                    event.event_type.as_str(),
                )?;
                add_product_delta(&mut product_deltas, &product_id, quantity, 0);
            }
            EventType::ProductShipped => {
                let product_id = req(&event.product_id, "product_id")?;
                let zone_id = req(&event.zone_id, "zone_id")?;
                let quantity = req_i32(event.quantity, "quantity")?;
                add_delta(
                    &mut zone_states,
                    &product_id,
                    &zone_id,
                    -quantity,
                    0,
                    event.event_type.as_str(),
                )?;
                add_product_delta(&mut product_deltas, &product_id, -quantity, 0);
            }
            EventType::ProductMoved => {
                let product_id = req(&event.product_id, "product_id")?;
                let from_zone_id = req(&event.from_zone_id, "from_zone_id")?;
                let to_zone_id = req(&event.to_zone_id, "to_zone_id")?;
                let quantity = req_i32(event.quantity, "quantity")?;
                add_delta(
                    &mut zone_states,
                    &product_id,
                    &from_zone_id,
                    -quantity,
                    0,
                    event.event_type.as_str(),
                )?;
                add_delta(
                    &mut zone_states,
                    &product_id,
                    &to_zone_id,
                    quantity,
                    0,
                    event.event_type.as_str(),
                )?;
            }
            EventType::ProductReserved => {
                let product_id = req(&event.product_id, "product_id")?;
                let zone_id = req(&event.zone_id, "zone_id")?;
                let quantity = req_i32(event.quantity, "quantity")?;
                add_delta(
                    &mut zone_states,
                    &product_id,
                    &zone_id,
                    -quantity,
                    quantity,
                    event.event_type.as_str(),
                )?;
                add_product_delta(&mut product_deltas, &product_id, -quantity, quantity);
            }
            EventType::ProductReleased => {
                let product_id = req(&event.product_id, "product_id")?;
                let zone_id = req(&event.zone_id, "zone_id")?;
                let quantity = req_i32(event.quantity, "quantity")?;
                add_delta(
                    &mut zone_states,
                    &product_id,
                    &zone_id,
                    quantity,
                    -quantity,
                    event.event_type.as_str(),
                )?;
                add_product_delta(&mut product_deltas, &product_id, quantity, -quantity);
            }
            EventType::InventoryCounted => {
                let product_id = req(&event.product_id, "product_id")?;
                let zone_id = req(&event.zone_id, "zone_id")?;
                let counted = req_i32(event.counted_quantity, "counted_quantity")?;
                let key = (product_id.clone(), zone_id.clone());
                let zone_state = zone_states
                    .get_mut(&key)
                    .ok_or_else(|| anyhow::anyhow!("zone state missing"))?;
                let delta = counted - zone_state.available;
                zone_state.available = counted;
                add_product_delta(&mut product_deltas, &product_id, delta, 0);
            }
            EventType::OrderCreated => {
                for item in req_items(event)? {
                    add_delta(
                        &mut zone_states,
                        &item.product_id,
                        &item.zone_id,
                        -item.quantity,
                        item.quantity,
                        event.event_type.as_str(),
                    )?;
                    add_product_delta(
                        &mut product_deltas,
                        &item.product_id,
                        -item.quantity,
                        item.quantity,
                    );
                }
            }
            EventType::OrderCompleted => {
                for item in req_items(event)? {
                    add_delta(
                        &mut zone_states,
                        &item.product_id,
                        &item.zone_id,
                        0,
                        -item.quantity,
                        event.event_type.as_str(),
                    )?;
                    add_product_delta(&mut product_deltas, &item.product_id, 0, -item.quantity);
                }
            }
        }

        let mut product_totals = HashMap::new();
        for product_id in product_deltas.keys() {
            let totals = self.read_product_totals(product_id).await?;
            product_totals.insert(product_id.clone(), totals);
        }

        let now_ms = Utc::now().timestamp_millis();
        let event_ts_ms = event.event_timestamp.timestamp_millis();
        let event_id = event.event_id.to_string();
        let supplier = if event.event_type == EventType::ProductReceived {
            event.supplier_id.clone()
        } else {
            None
        };
        let mut batch = Batch::new(BatchType::Logged);
        batch.set_consistency(Consistency::Quorum);
        let mut batch_values: Vec<Vec<Option<CqlValue>>> = Vec::new();
        for ((product_id, zone_id), state) in &zone_states {
            batch.append_statement(
                include_str!("../queries/update_inventory_by_product_zone.cql").trim(),
            );
            batch_values.push(vec![
                Some(CqlValue::Int(state.available)),
                Some(CqlValue::Int(state.reserved)),
                supplier.clone().map(CqlValue::Text),
                Some(CqlValue::Text(event_id.clone())),
                Some(CqlValue::BigInt(event_ts_ms)),
                Some(CqlValue::BigInt(now_ms)),
                Some(CqlValue::Text(product_id.clone())),
                Some(CqlValue::Text(zone_id.clone())),
            ]);
            batch.append_statement(include_str!("../queries/update_inventory_by_zone.cql").trim());
            batch_values.push(vec![
                Some(CqlValue::Int(state.available)),
                Some(CqlValue::Int(state.reserved)),
                supplier.clone().map(CqlValue::Text),
                Some(CqlValue::Text(event_id.clone())),
                Some(CqlValue::BigInt(event_ts_ms)),
                Some(CqlValue::BigInt(now_ms)),
                Some(CqlValue::Text(zone_id.clone())),
                Some(CqlValue::Text(product_id.clone())),
            ]);
            batch.append_statement(
                include_str!("../queries/update_product_event_versions.cql").trim(),
            );
            batch_values.push(vec![
                Some(CqlValue::BigInt(event_ts_ms)),
                Some(CqlValue::Text(event_id.clone())),
                Some(CqlValue::Int(1)),
                Some(CqlValue::BigInt(now_ms)),
                Some(CqlValue::Text(product_id.clone())),
                Some(CqlValue::Text(zone_id.clone())),
            ]);
        }
        for (product_id, (delta_available, delta_reserved)) in &product_deltas {
            let current = product_totals
                .get(product_id)
                .copied()
                .ok_or_else(|| anyhow::anyhow!("product totals missing"))?;
            let new_available = current.available + delta_available;
            let new_reserved = current.reserved + delta_reserved;
            if new_available < 0 || new_reserved < 0 {
                anyhow::bail!("event creates negative totals for product {product_id}");
            }
            batch.append_statement(
                include_str!("../queries/update_inventory_by_product.cql").trim(),
            );
            batch_values.push(vec![
                Some(CqlValue::Int(new_available)),
                Some(CqlValue::Int(new_reserved)),
                supplier.clone().map(CqlValue::Text),
                Some(CqlValue::Text(event_id.clone())),
                Some(CqlValue::BigInt(event_ts_ms)),
                Some(CqlValue::BigInt(now_ms)),
                Some(CqlValue::Text(product_id.clone())),
            ]);
        }
        if event.event_type == EventType::OrderCreated {
            let order_id = req(&event.order_id, "order_id")?;
            batch.append_statement(include_str!("../queries/insert_order.cql").trim());
            batch_values.push(vec![
                Some(CqlValue::Text(order_id.clone())),
                Some(CqlValue::BigInt(event_ts_ms)),
                Some(CqlValue::BigInt(now_ms)),
                Some(CqlValue::Text(event_id.clone())),
                Some(CqlValue::BigInt(event_ts_ms)),
            ]);
            for item in req_items(event)? {
                batch.append_statement(include_str!("../queries/insert_order_item.cql").trim());
                batch_values.push(vec![
                    Some(CqlValue::Text(order_id.clone())),
                    Some(CqlValue::Text(item.product_id.clone())),
                    Some(CqlValue::Text(item.zone_id.clone())),
                    Some(CqlValue::Int(item.quantity)),
                    Some(CqlValue::BigInt(event_ts_ms)),
                ]);
            }
        }
        if event.event_type == EventType::OrderCompleted {
            let order_id = req(&event.order_id, "order_id")?;
            batch.append_statement(include_str!("../queries/update_order_completed.cql").trim());
            batch_values.push(vec![
                Some(CqlValue::BigInt(now_ms)),
                Some(CqlValue::Text(event_id.clone())),
                Some(CqlValue::BigInt(event_ts_ms)),
                Some(CqlValue::Text(order_id)),
            ]);
        }
        batch.append_statement(include_str!("../queries/insert_processed_event.cql").trim());
        batch_values.push(vec![
            Some(CqlValue::Text(event_id)),
            Some(CqlValue::Text(event.event_type.as_str().to_string())),
            Some(CqlValue::BigInt(event_ts_ms)),
            Some(CqlValue::Int(kafka_partition)),
            Some(CqlValue::BigInt(kafka_offset)),
            Some(CqlValue::BigInt(now_ms)),
        ]);
        self.session.batch(&batch, &batch_values).await?;
        Ok(ProcessOutcome::Applied)
    }

    async fn mark_processed(
        &self,
        event: &WarehouseEventPayload,
        kafka_partition: i32,
        kafka_offset: i64,
    ) -> anyhow::Result<()> {
        let now_ms = Utc::now().timestamp_millis();
        let event_ts_ms = event.event_timestamp.timestamp_millis();
        let mut stmt = Statement::new(include_str!("../queries/insert_processed_event.cql").trim());
        stmt.set_consistency(Consistency::Quorum);
        self.session
            .query_unpaged(
                stmt,
                (
                    event.event_id.to_string(),
                    event.event_type.as_str().to_string(),
                    event_ts_ms,
                    kafka_partition,
                    kafka_offset,
                    now_ms,
                ),
            )
            .await?;
        Ok(())
    }

    async fn is_duplicate(&self, event_id: &str) -> anyhow::Result<bool> {
        let mut stmt =
            Statement::new(include_str!("../queries/select_processed_event_by_id.cql").trim());
        stmt.set_consistency(self.read_consistency);
        let rows = self
            .session
            .query_unpaged(stmt, (event_id.to_string(),))
            .await?
            .into_rows_result()?;
        Ok(rows.rows_num() > 0)
    }

    async fn is_stale(
        &self,
        product_id: &str,
        zone_id: &str,
        event_ts_ms: i64,
    ) -> anyhow::Result<bool> {
        let mut stmt =
            Statement::new(include_str!("../queries/select_last_event_timestamp.cql").trim());
        stmt.set_consistency(self.read_consistency);
        let result = self
            .session
            .query_unpaged(stmt, (product_id.to_string(), zone_id.to_string()))
            .await?;
        let Ok(rows_result) = result.into_rows_result() else {
            return Ok(false);
        };
        for row in rows_result.rows::<(Option<i64>,)>()? {
            let (last_ts,) = row?;
            if let Some(last_ts) = last_ts {
                return Ok(event_ts_ms <= last_ts);
            }
        }
        Ok(false)
    }

    async fn read_zone_state(&self, product_id: &str, zone_id: &str) -> anyhow::Result<ZoneState> {
        let mut stmt = Statement::new(include_str!("../queries/select_zone_state.cql").trim());
        stmt.set_consistency(self.read_consistency);
        let result = self
            .session
            .query_unpaged(stmt, (product_id.to_string(), zone_id.to_string()))
            .await?;
        if let Ok(rows_result) = result.into_rows_result() {
            for row in rows_result.rows::<(Option<i32>, Option<i32>)>()? {
                let (available, reserved) = row?;
                return Ok(ZoneState {
                    available: available.unwrap_or(0),
                    reserved: reserved.unwrap_or(0),
                });
            }
        }
        Ok(ZoneState::default())
    }

    async fn read_product_totals(&self, product_id: &str) -> anyhow::Result<ProductTotals> {
        let mut stmt = Statement::new(include_str!("../queries/select_product_totals.cql").trim());
        stmt.set_consistency(self.read_consistency);
        let result = self
            .session
            .query_unpaged(stmt, (product_id.to_string(),))
            .await?;
        if let Ok(rows_result) = result.into_rows_result() {
            for row in rows_result.rows::<(Option<i32>, Option<i32>)>()? {
                let (available, reserved) = row?;
                return Ok(ProductTotals {
                    available: available.unwrap_or(0),
                    reserved: reserved.unwrap_or(0),
                });
            }
        }
        Ok(ProductTotals::default())
    }
}

fn req(value: &Option<String>, field: &str) -> anyhow::Result<String> {
    value
        .as_ref()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .ok_or_else(|| anyhow::anyhow!("{field} is required"))
}

fn req_i32(value: Option<i32>, field: &str) -> anyhow::Result<i32> {
    value.ok_or_else(|| anyhow::anyhow!("{field} is required"))
}

fn req_items(event: &WarehouseEventPayload) -> anyhow::Result<&Vec<OrderItem>> {
    event
        .items
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("items are required"))
}

fn add_delta(
    states: &mut HashMap<(String, String), ZoneState>,
    product_id: &str,
    zone_id: &str,
    delta_available: i32,
    delta_reserved: i32,
    event_type: &str,
) -> anyhow::Result<()> {
    let key = (product_id.to_string(), zone_id.to_string());
    let state = states
        .get_mut(&key)
        .ok_or_else(|| anyhow::anyhow!("missing zone state for {} {}", product_id, zone_id))?;
    let new_available = state.available + delta_available;
    let new_reserved = state.reserved + delta_reserved;
    if new_available < 0 || new_reserved < 0 {
        anyhow::bail!("event {event_type} creates negative inventory");
    }
    state.available = new_available;
    state.reserved = new_reserved;
    Ok(())
}

fn add_product_delta(
    product_deltas: &mut HashMap<String, (i32, i32)>,
    product_id: &str,
    delta_available: i32,
    delta_reserved: i32,
) {
    let entry = product_deltas
        .entry(product_id.to_string())
        .or_insert((0, 0));
    entry.0 += delta_available;
    entry.1 += delta_reserved;
}

fn touched_entities(event: &WarehouseEventPayload) -> anyhow::Result<Vec<(String, String)>> {
    let mut touched = Vec::new();
    match event.event_type {
        EventType::ProductReceived
        | EventType::ProductShipped
        | EventType::ProductReserved
        | EventType::ProductReleased
        | EventType::InventoryCounted => {
            touched.push((
                req(&event.product_id, "product_id")?,
                req(&event.zone_id, "zone_id")?,
            ));
        }
        EventType::ProductMoved => {
            let product_id = req(&event.product_id, "product_id")?;
            touched.push((
                product_id.clone(),
                req(&event.from_zone_id, "from_zone_id")?,
            ));
            touched.push((product_id, req(&event.to_zone_id, "to_zone_id")?));
        }
        EventType::OrderCreated | EventType::OrderCompleted => {
            for item in req_items(event)? {
                touched.push((item.product_id.clone(), item.zone_id.clone()));
            }
        }
    }
    Ok(touched)
}
