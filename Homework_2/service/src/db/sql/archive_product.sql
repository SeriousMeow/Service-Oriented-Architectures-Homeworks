UPDATE products
SET
    status = 'ARCHIVED'::product_status,
    updated_at = NOW()
WHERE id = $1
