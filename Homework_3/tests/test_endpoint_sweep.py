from __future__ import annotations

import uuid

import requests

from conftest import (
    assert_booking,
    assert_error,
    assert_flight_item,
)


def test_get_flights_happy_and_bad_request(reset_and_seed, base_url: str, booking_client) -> None:
    # Happy path: search by origin/destination.
    resp = booking_client.get_flights(origin="SVO", destination="LED")
    assert resp.status_code == 200, resp.text
    data = resp.json()
    assert isinstance(data.get("items"), list)
    assert data["items"], "expected at least one flight in search results"
    assert_flight_item(data["items"][0])

    # Error path: missing required query param -> 400 with an error payload.
    r2 = requests.get(f"{base_url}/flights", params={"origin": "SVO"}, timeout=5)
    assert r2.status_code == 400, r2.text


def test_get_flight_by_id_happy_and_not_found(reset_and_seed, booking_client) -> None:
    flight_id = reset_and_seed

    # Happy path.
    resp = booking_client.get_flight_by_id(flight_id)
    assert resp.status_code == 200, resp.text
    item = resp.json()
    assert_flight_item(item, expected_id=flight_id)

    # Error path: unknown flight id -> 404.
    unknown_id = uuid.uuid4()
    r2 = booking_client.get_flight_by_id(unknown_id)
    assert_error(r2, 404, "FLIGHT_NOT_FOUND")


def test_bookings_crud_and_list(reset_and_seed, base_url: str, booking_client) -> None:
    flight_id = reset_and_seed
    user_id = uuid.uuid4()

    # Create booking (POST /bookings).
    resp = booking_client.post_booking(
        user_id=user_id,
        flight_id=flight_id,
        passenger_name="John Doe",
        passenger_email="john.doe@example.com",
        seat_count=1,
    )
    assert resp.status_code == 201, resp.text
    booking = assert_booking(resp, expected_status="CONFIRMED")
    booking_id = uuid.UUID(booking["id"])

    # Error path: invalid payload (seat_count <= 0) -> 400.
    bad_payload = {
        "user_id": str(uuid.uuid4()),
        "flight_id": str(flight_id),
        "passenger_name": "Jane Doe",
        "passenger_email": "jane.doe@example.com",
        "seat_count": 0,
    }
    r2 = requests.post(f"{base_url}/bookings", json=bad_payload, timeout=10)
    assert r2.status_code == 400, r2.text

    # Get booking by id (GET /bookings/{id}).
    r3 = booking_client.get_booking_by_id(booking_id)
    assert r3.status_code == 200, r3.text
    booking_by_id = assert_booking(r3)
    assert booking_by_id["id"] == str(booking_id)

    # List bookings by user (GET /bookings?user_id=).
    r4 = booking_client.list_bookings(user_id)
    assert r4.status_code == 200, r4.text
    data = r4.json()
    items = data.get("items", [])
    assert isinstance(items, list)
    assert any(item["id"] == str(booking_id) for item in items)

    # Error path: missing user_id -> 400.
    r5 = requests.get(f"{base_url}/bookings", timeout=5)
    assert r5.status_code == 400, r5.text


def test_cancel_booking_happy_conflict_and_not_found(reset_and_seed, booking_client) -> None:
    flight_id = reset_and_seed
    user_id = uuid.uuid4()

    # Create a CONFIRMED booking to cancel.
    resp = booking_client.post_booking(
        user_id=user_id,
        flight_id=flight_id,
        passenger_name="Cancel Me",
        passenger_email="cancel.me@example.com",
        seat_count=1,
    )
    assert resp.status_code == 201, resp.text
    booking = assert_booking(resp, expected_status="CONFIRMED")
    booking_id = uuid.UUID(booking["id"])

    # Happy path: first cancel succeeds, booking moves to CANCELLED.
    r1 = booking_client.cancel_booking(booking_id)
    assert r1.status_code == 200, r1.text
    cancelled = assert_booking(r1, expected_status="CANCELLED")
    assert cancelled["id"] == str(booking_id)

    # Error path: second cancel of the same booking -> 409 conflict.
    r2 = booking_client.cancel_booking(booking_id)
    assert_error(r2, 409, "BOOKING_CANNOT_BE_CANCELLED")

    # Error path: cancelling a non-existent booking id -> 404.
    unknown_id = uuid.uuid4()
    r3 = booking_client.cancel_booking(unknown_id)
    assert_error(r3, 404, "BOOKING_NOT_FOUND")

