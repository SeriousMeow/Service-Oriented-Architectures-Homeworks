UPDATE warehouse.inventory_by_zone
SET available_quantity = ?,
    reserved_quantity = ?,
    supplier_id = ?,
    last_event_id = ?,
    last_event_timestamp = ?,
    updated_at = ?
WHERE zone_id = ?
AND product_id = ?;
