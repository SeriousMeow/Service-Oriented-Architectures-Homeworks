INSERT INTO products (name, description, price, stock, category, status, seller_id)
VALUES ($1, $2, $3, $4, $5, $6::product_status, $7)
