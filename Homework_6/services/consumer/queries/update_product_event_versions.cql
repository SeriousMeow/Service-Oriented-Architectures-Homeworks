UPDATE warehouse.product_event_versions
SET last_event_timestamp = ?,
    last_event_id = ?,
    schema_version = ?,
    updated_at = ?
WHERE product_id = ?
AND zone_id = ?;
