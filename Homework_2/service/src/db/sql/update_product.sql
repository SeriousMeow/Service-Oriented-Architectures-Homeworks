UPDATE products
SET
    name = COALESCE($2, name),
    description = COALESCE($3, description),
    price = COALESCE($4, price),
    stock = COALESCE($5, stock),
    category = COALESCE($6, category),
    status = COALESCE($7::product_status, status),
    updated_at = NOW()
WHERE id = $1
