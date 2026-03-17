SELECT
    id,
    name,
    description,
    price,
    stock, category,
    status,
    created_at,
    updated_at,
    seller_id
FROM products 
WHERE
    ($1::product_status IS NULL OR status = $1::product_status)
    AND ($2::TEXT IS NULL OR category = $2)
ORDER BY id 
LIMIT $3 OFFSET $4
