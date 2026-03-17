SELECT id, order_id, product_id, quantity, price_at_order
FROM order_items
WHERE order_id = $1
