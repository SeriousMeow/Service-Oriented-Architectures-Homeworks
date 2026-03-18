import uuid

import pytest

from conftest import assert_error, assert_flight_item


def test_get_flights_returns_seeded(reset_and_seed: uuid.UUID, booking_client, seed_cfg):
    flight_id = reset_and_seed
    resp = booking_client.get_flights(seed_cfg.origin, seed_cfg.destination)
    assert resp.status_code == 200, resp.text
    data = resp.json()
    assert "items" in data
    assert isinstance(data["items"], list)
    assert any(item.get("id") == str(flight_id) for item in data["items"]), data


def test_get_flights_by_id_ok(reset_and_seed: uuid.UUID, booking_client):
    flight_id = reset_and_seed
    resp = booking_client.get_flight_by_id(flight_id)
    assert resp.status_code == 200, resp.text
    item = resp.json()
    assert_flight_item(item, expected_id=flight_id)


def test_get_flights_by_id_not_found(booking_client):
    resp = booking_client.get_flight_by_id(uuid.UUID("00000000-0000-0000-0000-000000000000"))
    assert_error(resp, 404, "FLIGHT_NOT_FOUND")


def test_get_flights_validation_error(booking_client):
    # origin/destination must be IATA codes like "SVO"
    resp = booking_client.get_flights("sv", "LED")
    assert resp.status_code == 400, resp.text

