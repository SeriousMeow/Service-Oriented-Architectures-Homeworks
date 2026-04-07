SELECT
    id,
    booking_id,
    flight_id,
    seat_count,
    status::text AS status,
    created_at,
    updated_at
FROM seat_reservations
WHERE booking_id = $1
FOR UPDATE;
