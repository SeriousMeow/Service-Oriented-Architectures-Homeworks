SELECT
    id,
    user_id,
    flight_id,
    passenger_name,
    passenger_email,
    seat_count,
    price_currency,
    total_price_minor,
    status,
    created_at,
    updated_at
FROM bookings
WHERE user_id = $1
ORDER BY created_at DESC;
