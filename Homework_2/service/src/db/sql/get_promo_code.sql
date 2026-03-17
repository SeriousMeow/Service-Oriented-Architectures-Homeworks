SELECT id, code, discount_type, discount_value, min_order_amount, 
       max_uses, current_uses, valid_from, valid_until, active
FROM promo_codes
WHERE code = $1
