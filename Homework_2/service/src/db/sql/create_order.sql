INSERT INTO orders (user_id, status, promo_code_id, total_amount, discount_amount)
VALUES ($1, $2, $3, $4, $5)
RETURNING id, user_id, status, promo_code_id, total_amount, discount_amount, created_at, updated_at, (SELECT code FROM promo_codes WHERE id = orders.promo_code_id) AS promo_code
