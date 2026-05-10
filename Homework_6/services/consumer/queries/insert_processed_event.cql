INSERT INTO warehouse.processed_events
    (event_id, event_type, event_timestamp, kafka_partition, kafka_offset, processed_at)
VALUES
    (?, ?, ?, ?, ?, ?);
