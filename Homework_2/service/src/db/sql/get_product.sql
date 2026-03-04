SELECT
    id,
    name,
    description,
    price,
    stock,
    category,
    status,
    created_at,
    updated_at,
    seller_id
FROM
    products
WHERE
    id = $1
