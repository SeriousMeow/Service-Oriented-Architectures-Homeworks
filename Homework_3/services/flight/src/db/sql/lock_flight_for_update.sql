SELECT
    status,
    available_seats
FROM flights
WHERE id = $1
FOR UPDATE;
