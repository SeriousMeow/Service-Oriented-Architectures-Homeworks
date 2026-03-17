SELECT
    COUNT(*)::BIGINT
FROM products
WHERE
    ($1::product_status IS NULL OR status = $1::product_status)
    AND ($2::TEXT IS NULL OR category = $2)
