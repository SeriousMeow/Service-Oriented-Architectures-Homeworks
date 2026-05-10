UPDATE warehouse.inventory_by_product_zone
SET available_quantity = ?,
    reserved_quantity = ?,
    supplier_id = ?,
    last_event_id = ?,
    last_event_timestamp = ?,
    updated_at = ?
WHERE product_id = ?
AND zone_id = ?;
