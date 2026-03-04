UPDATE promo_codes
SET current_uses = current_uses - 1
WHERE id = $1 AND current_uses > 0
