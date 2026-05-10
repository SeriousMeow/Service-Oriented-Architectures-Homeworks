CREATE KEYSPACE IF NOT EXISTS warehouse
WITH replication = {'class': 'NetworkTopologyStrategy', 'datacenter1': 3};

CREATE TABLE IF NOT EXISTS warehouse.inventory_by_product_zone (
  product_id text,
  zone_id text,
  available_quantity int,
  reserved_quantity int,
  supplier_id text,
  last_event_id text,
  last_event_timestamp timestamp,
  updated_at timestamp,
  PRIMARY KEY ((product_id), zone_id)
);

CREATE TABLE IF NOT EXISTS warehouse.inventory_by_product (
  product_id text PRIMARY KEY,
  total_available int,
  total_reserved int,
  supplier_id text,
  last_event_id text,
  last_event_timestamp timestamp,
  updated_at timestamp
);

CREATE TABLE IF NOT EXISTS warehouse.inventory_by_zone (
  zone_id text,
  product_id text,
  available_quantity int,
  reserved_quantity int,
  supplier_id text,
  last_event_id text,
  last_event_timestamp timestamp,
  updated_at timestamp,
  PRIMARY KEY ((zone_id), product_id)
);

CREATE TABLE IF NOT EXISTS warehouse.processed_events (
  event_id text PRIMARY KEY,
  event_type text,
  event_timestamp timestamp,
  kafka_partition int,
  kafka_offset bigint,
  processed_at timestamp
);

CREATE TABLE IF NOT EXISTS warehouse.product_event_versions (
  product_id text,
  zone_id text,
  last_event_timestamp timestamp,
  last_event_id text,
  schema_version int,
  updated_at timestamp,
  PRIMARY KEY ((product_id), zone_id)
);

CREATE TABLE IF NOT EXISTS warehouse.orders (
  order_id text PRIMARY KEY,
  status text,
  created_at timestamp,
  updated_at timestamp,
  event_id text,
  event_timestamp timestamp
);

CREATE TABLE IF NOT EXISTS warehouse.order_items (
  order_id text,
  product_id text,
  zone_id text,
  quantity int,
  created_at timestamp,
  PRIMARY KEY ((order_id), product_id, zone_id)
);
