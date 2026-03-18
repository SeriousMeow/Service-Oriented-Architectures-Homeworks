SELECT
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
    updated_at
FROM flights
WHERE origin = $1
  AND destination = $2
  AND departure_time::date = $3
  AND status = 'SCHEDULED'
ORDER BY departure_time ASC;
