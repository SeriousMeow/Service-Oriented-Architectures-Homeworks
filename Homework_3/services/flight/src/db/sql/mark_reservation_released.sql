UPDATE seat_reservations
SET
    status = 'RELEASED',
    updated_at = now()
WHERE id = $1
RETURNING
    id,
    booking_id,
    flight_id,
    seat_count,
    status::text AS status,
    created_at,
    updated_at;
