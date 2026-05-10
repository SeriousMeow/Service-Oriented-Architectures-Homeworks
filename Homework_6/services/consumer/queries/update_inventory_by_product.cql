UPDATE warehouse.inventory_by_product
SET total_available = ?,
    total_reserved = ?,
    supplier_id = ?,
    last_event_id = ?,
    last_event_timestamp = ?,
    updated_at = ?
WHERE product_id = ?;
