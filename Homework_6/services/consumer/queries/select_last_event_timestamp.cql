SELECT toUnixTimestamp(last_event_timestamp)
FROM warehouse.product_event_versions
WHERE product_id = ?
AND zone_id = ?
