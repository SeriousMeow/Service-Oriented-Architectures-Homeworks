INSERT INTO seat_reservations (
    booking_id,
    flight_id,
    seat_count,
    status
)
VALUES ($1, $2, $3, 'ACTIVE')
RETURNING
    id,
    booking_id,
    flight_id,
    seat_count,
    status::text AS status,
    created_at,
    updated_at;
