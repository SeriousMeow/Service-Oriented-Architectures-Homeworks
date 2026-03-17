SELECT id, email, password_hash, role, created_at, updated_at
FROM users
WHERE email = $1
