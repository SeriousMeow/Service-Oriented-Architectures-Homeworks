INSERT INTO order_items (order_id, product_id, quantity, price_at_order)
VALUES ($1, $2, $3, $4)
RETURNING id, order_id, product_id, quantity, price_at_order
