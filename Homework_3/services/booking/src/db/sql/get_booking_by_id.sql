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
WHERE id = $1;
