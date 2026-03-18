CREATE TYPE flight_status AS ENUM (
    'SCHEDULED',
    'DEPARTED',
    'CANCELLED',
    'COMPLETED'
);

CREATE TYPE seat_reservation_status AS ENUM (
    'ACTIVE',
    'RELEASED',
    'EXPIRED'
);

CREATE TABLE flights (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    flight_number varchar(16) NOT NULL,
    airline varchar(128) NOT NULL,
    origin char(3) NOT NULL CHECK (origin ~ '^[A-Z]{3}$'),
    destination char(3) NOT NULL CHECK (destination ~ '^[A-Z]{3}$'),
    departure_date date GENERATED ALWAYS AS ((departure_time AT TIME ZONE 'UTC')::date) STORED,
    departure_time timestamptz NOT NULL,
    arrival_time timestamptz NOT NULL,
    total_seats integer NOT NULL CHECK (total_seats > 0),
    available_seats integer NOT NULL,
    price_currency char(3) NOT NULL CHECK (price_currency ~ '^[A-Z]{3}$'),
    price_minor_units bigint NOT NULL CHECK (price_minor_units > 0),
    status flight_status NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT chk_flights_departure_before_arrival CHECK (departure_time < arrival_time),
    CONSTRAINT chk_flights_available_seats_range CHECK (available_seats >= 0 AND available_seats <= total_seats)
);

CREATE UNIQUE INDEX ux_flights_flight_number_departure_date
    ON flights (flight_number, departure_date);

CREATE INDEX ix_flights_route_departure_status
    ON flights (origin, destination, departure_time, status);

CREATE TABLE seat_reservations (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    booking_id uuid NOT NULL UNIQUE,
    flight_id uuid NOT NULL REFERENCES flights(id) ON DELETE RESTRICT,
    seat_count integer NOT NULL CHECK (seat_count > 0),
    status seat_reservation_status NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX ix_seat_reservations_flight_status
    ON seat_reservations (flight_id, status);
