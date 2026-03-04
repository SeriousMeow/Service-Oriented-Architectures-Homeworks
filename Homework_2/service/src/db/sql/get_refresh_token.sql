SELECT id, user_id, token, expires_at, created_at
FROM refresh_tokens
WHERE token = $1 AND expires_at > NOW()
