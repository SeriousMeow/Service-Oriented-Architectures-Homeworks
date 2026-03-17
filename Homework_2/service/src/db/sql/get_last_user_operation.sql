SELECT id, user_id, operation_type, created_at
FROM user_operations
WHERE user_id = $1 AND operation_type = $2
ORDER BY created_at DESC
LIMIT 1
