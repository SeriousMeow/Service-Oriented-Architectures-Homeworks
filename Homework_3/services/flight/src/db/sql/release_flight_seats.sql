UPDATE flights
SET
    available_seats = available_seats + $1,
    updated_at = now()
WHERE id = $2
RETURNING
    id,
    flight_number,
    airline,
    origin,
    destination,
    departure_time,
    arrival_time,
    total_seats,
    available_seats,
    price_currency,
    price_minor_units,
    status::text AS status,
    created_at,
    updated_at;
