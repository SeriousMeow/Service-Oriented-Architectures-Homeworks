CREATE TABLE IF NOT EXISTS movie_events_queue
(
    event_id String,
    user_id String,
    movie_id String,
    event_type String,
    `timestamp` DateTime64(3, 'UTC'),
    device_type String,
    session_id String,
    progress_seconds Int32
)
ENGINE = Kafka
SETTINGS
    kafka_broker_list = 'kafka:29092',
    kafka_topic_list = 'movie-events',
    kafka_group_name = 'clickhouse_movie_events',
    kafka_format = 'AvroConfluent',
    format_avro_schema_registry_url = 'http://schema-registry:8081',
    kafka_num_consumers = 1,
    kafka_flush_interval_ms = 750;

CREATE TABLE IF NOT EXISTS movie_events
(
    event_id String,
    user_id String,
    movie_id String,
    event_type LowCardinality(String),
    `timestamp` DateTime64(3, 'UTC'),
    device_type LowCardinality(String),
    session_id String,
    progress_seconds Int32
)
ENGINE = MergeTree
PARTITION BY toYYYYMMDD(timestamp)
ORDER BY (user_id, timestamp, event_id);

CREATE MATERIALIZED VIEW IF NOT EXISTS movie_events_mv TO movie_events AS
SELECT
    event_id,
    user_id,
    movie_id,
    event_type,
    `timestamp`,
    device_type,
    session_id,
    progress_seconds
FROM movie_events_queue;
