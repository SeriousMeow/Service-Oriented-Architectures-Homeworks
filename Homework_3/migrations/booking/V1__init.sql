CREATE TYPE booking_status AS ENUM (
    'CONFIRMED',
    'CANCELLED'
);

CREATE TABLE bookings (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id uuid NOT NULL,
    flight_id uuid NOT NULL,
    passenger_name varchar(120) NOT NULL CHECK (btrim(passenger_name) <> ''),
    passenger_email varchar(255) NOT NULL CHECK (passenger_email ~* '^[A-Z0-9._%+\-]+@[A-Z0-9.\-]+\.[A-Z]{2,}$'),
    seat_count integer NOT NULL CHECK (seat_count > 0),
    price_currency char(3) NOT NULL CHECK (price_currency ~ '^[A-Z]{3}$'),
    total_price_minor bigint NOT NULL CHECK (total_price_minor > 0),
    status booking_status NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX ix_bookings_user_created
    ON bookings (user_id, created_at DESC);

CREATE INDEX ix_bookings_flight
    ON bookings (flight_id);
