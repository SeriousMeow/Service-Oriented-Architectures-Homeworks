INSERT INTO warehouse.orders
    (order_id, status, created_at, updated_at, event_id, event_timestamp)
VALUES
    (?, 'CREATED', ?, ?, ?, ?);
