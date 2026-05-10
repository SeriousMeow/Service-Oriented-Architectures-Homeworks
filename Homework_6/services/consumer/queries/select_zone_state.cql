SELECT available_quantity, reserved_quantity
FROM warehouse.inventory_by_product_zone
WHERE product_id = ?
AND zone_id = ?
