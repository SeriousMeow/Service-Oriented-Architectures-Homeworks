import concurrent.futures
import uuid

import psycopg
import pytest

from conftest import assert_error


@pytest.mark.req5
def test_last_seat_no_oversell(reset_and_seed, booking_client, pg_flight_conninfo, seed_cfg):
    # Re-seed with a single seat to force contention.
    with psycopg.connect(pg_flight_conninfo) as conn:
        with conn.cursor() as cur:
            cur.execute("TRUNCATE seat_reservations RESTART IDENTITY CASCADE;")
            cur.execute("TRUNCATE flights RESTART IDENTITY CASCADE;")
            cur.execute(
                """
                INSERT INTO flights (
                  flight_number, airline, origin, destination,
                  departure_time, arrival_time,
                  total_seats, available_seats,
                  price_currency, price_minor_units, status
                ) VALUES (
                  %s, %s, %s, %s,
                  %s, %s,
                  1, 1,
                  %s, %s, 'SCHEDULED'
                )
                RETURNING id;
                """,
                (
                    seed_cfg.flight_number,
                    seed_cfg.airline,
                    seed_cfg.origin,
                    seed_cfg.destination,
                    seed_cfg.departure_time,
                    seed_cfg.arrival_time,
                    seed_cfg.price_currency,
                    seed_cfg.price_minor_units,
                ),
            )
            (flight_id,) = cur.fetchone()
        conn.commit()

    def do_book() -> int:
        resp = booking_client.post_booking(
            user_id=uuid.uuid4(),
            flight_id=flight_id,
            passenger_name="Test User",
            passenger_email="test@example.com",
            seat_count=1,
        )
        return resp.status_code

    with concurrent.futures.ThreadPoolExecutor(max_workers=2) as ex:
        results = list(ex.map(lambda _: do_book(), range(2)))

    assert sorted(results) == [201, 409], results

    with psycopg.connect(pg_flight_conninfo) as conn:
        with conn.cursor() as cur:
            cur.execute("SELECT available_seats FROM flights WHERE id=%s;", (flight_id,))
            (available,) = cur.fetchone()
            assert available == 0
            cur.execute("SELECT COUNT(*) FROM seat_reservations WHERE flight_id=%s AND status='ACTIVE';", (flight_id,))
            (active_count,) = cur.fetchone()
            assert active_count == 1

