UPDATE bookings
SET
    status = $2::booking_status,
    updated_at = now()
WHERE id = $1
RETURNING
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
    updated_at;
