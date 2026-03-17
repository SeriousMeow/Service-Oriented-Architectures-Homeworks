SELECT o.id, o.user_id, o.status, o.promo_code_id, o.total_amount, 
       o.discount_amount, o.created_at, o.updated_at, pc.code as promo_code
FROM orders o
LEFT JOIN promo_codes pc ON o.promo_code_id = pc.id
WHERE o.id = $1
