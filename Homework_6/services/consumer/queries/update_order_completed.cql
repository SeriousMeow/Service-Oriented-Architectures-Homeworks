UPDATE warehouse.orders
SET status = 'COMPLETED',
    updated_at = ?,
    event_id = ?,
    event_timestamp = ?
WHERE order_id = ?;
