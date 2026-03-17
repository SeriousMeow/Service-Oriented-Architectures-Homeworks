UPDATE orders
SET status = $2, updated_at = NOW()
WHERE id = $1
RETURNING id, user_id, status, promo_code_id, total_amount, discount_amount, created_at, updated_at, (SELECT code FROM promo_codes WHERE id = orders.promo_code_id) AS promo_code
