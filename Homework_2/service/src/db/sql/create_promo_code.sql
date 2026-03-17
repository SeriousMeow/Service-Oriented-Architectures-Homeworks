INSERT INTO promo_codes (code, discount_type, discount_value, min_order_amount, max_uses, current_uses, valid_from, valid_until, active)
VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
RETURNING id, code, discount_type, discount_value, min_order_amount, max_uses, current_uses, valid_from, valid_until, active
