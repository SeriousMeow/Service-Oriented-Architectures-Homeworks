SELECT status::text AS status, available_seats
FROM flights
WHERE id = $1;
