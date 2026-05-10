SELECT total_available, total_reserved
FROM warehouse.inventory_by_product
WHERE product_id = ?
