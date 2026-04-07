import time
import uuid

import pytest


def _search_key(origin: str, destination: str) -> bytes:
    return f"search:{origin}:{destination}:no-date".encode()


@pytest.mark.req7
def test_cache_keys_ttl_and_hit(redis_client, reset_and_seed: uuid.UUID, booking_client, seed_cfg):
    flight_id = reset_and_seed

    flight_key = f"flight:{flight_id}".encode()
    search_key = _search_key(seed_cfg.origin, seed_cfg.destination)

    redis_client.delete(flight_key)
    redis_client.delete(search_key)

    # First request should populate cache.
    r1 = booking_client.get_flights(seed_cfg.origin, seed_cfg.destination)
    assert r1.status_code == 200
    r2 = booking_client.get_flight_by_id(flight_id)
    assert r2.status_code == 200

    assert redis_client.exists(search_key) == 1
    assert redis_client.exists(flight_key) == 1

    ttl_search = redis_client.ttl(search_key)
    ttl_flight = redis_client.ttl(flight_key)
    assert ttl_search > 0
    assert ttl_flight > 0

    # Second request should not remove keys and TTL should be decreasing (allow small race).
    time.sleep(1.0)
    r3 = booking_client.get_flights(seed_cfg.origin, seed_cfg.destination)
    assert r3.status_code == 200
    r4 = booking_client.get_flight_by_id(flight_id)
    assert r4.status_code == 200

    ttl_search2 = redis_client.ttl(search_key)
    ttl_flight2 = redis_client.ttl(flight_key)
    assert 0 < ttl_search2 <= ttl_search
    assert 0 < ttl_flight2 <= ttl_flight


@pytest.mark.req7
def test_cache_invalidation_after_mutation(redis_client, reset_and_seed: uuid.UUID, booking_client, seed_cfg):
    flight_id = reset_and_seed

    flight_key = f"flight:{flight_id}".encode()
    search_key = _search_key(seed_cfg.origin, seed_cfg.destination)

    # Prime cache.
    r_search = booking_client.get_flights(seed_cfg.origin, seed_cfg.destination)
    assert r_search.status_code == 200
    r_get = booking_client.get_flight_by_id(flight_id)
    assert r_get.status_code == 200
    assert redis_client.exists(flight_key) == 1
    before_available = r_get.json()["available_seats"]

    # Mutation (ReserveSeats) should invalidate.
    create = booking_client.post_booking(
        user_id=uuid.uuid4(),
        flight_id=flight_id,
        passenger_name="Test User",
        passenger_email="test@example.com",
        seat_count=1,
    )
    assert create.status_code == 201, create.text

    # Invalidation is after commit; verify the observable API state is fresh (no stale cache).
    r_get2 = booking_client.get_flight_by_id(flight_id)
    assert r_get2.status_code == 200
    after_available = r_get2.json()["available_seats"]
    assert after_available == before_available - 1

    r_search2 = booking_client.get_flights(seed_cfg.origin, seed_cfg.destination)
    assert r_search2.status_code == 200
    items = r_search2.json()["items"]
    found = [it for it in items if it.get("id") == str(flight_id)]
    assert len(found) == 1
    assert found[0]["available_seats"] == after_available

    # Next reads should repopulate.
    assert booking_client.get_flights(seed_cfg.origin, seed_cfg.destination).status_code == 200
    assert booking_client.get_flight_by_id(flight_id).status_code == 200
    assert redis_client.exists(flight_key) == 1

