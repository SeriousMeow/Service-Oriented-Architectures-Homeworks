import uuid

import pytest

from conftest import assert_booking, assert_error


def test_post_booking_create_get_list_cancel_happy_path(reset_and_seed: uuid.UUID, booking_client):
    flight_id = reset_and_seed
    user_id = uuid.uuid4()

    create = booking_client.post_booking(
        user_id=user_id,
        flight_id=flight_id,
        passenger_name="Test User",
        passenger_email="test@example.com",
        seat_count=2,
    )
    assert create.status_code == 201, create.text
    booking = assert_booking(create, expected_status="CONFIRMED")
    booking_id = uuid.UUID(booking["id"])

    get_by_id = booking_client.get_booking_by_id(booking_id)
    assert get_by_id.status_code == 200, get_by_id.text
    assert_booking(get_by_id, expected_status="CONFIRMED")

    listing = booking_client.list_bookings(user_id)
    assert listing.status_code == 200, listing.text
    items = listing.json()["items"]
    assert any(item["id"] == str(booking_id) for item in items), items

    cancel = booking_client.cancel_booking(booking_id)
    assert cancel.status_code == 200, cancel.text
    assert_booking(cancel, expected_status="CANCELLED")

    cancel_again = booking_client.cancel_booking(booking_id)
    assert_error(cancel_again, 409, "BOOKING_CANNOT_BE_CANCELLED")


def test_cancel_not_found(booking_client):
    resp = booking_client.cancel_booking(uuid.UUID("00000000-0000-0000-0000-000000000000"))
    assert_error(resp, 404, "BOOKING_NOT_FOUND")


def test_cancel_invalid_uuid(base_url, http_session):
    resp = http_session.post(f"{base_url}/bookings/not-a-uuid/cancel", timeout=5)
    assert resp.status_code == 400


def test_create_booking_not_enough_seats(reset_and_seed: uuid.UUID, booking_client, pg_flight_conninfo):
    # Create a small flight with only 1 seat and try to book 2.
    flight_id = reset_and_seed
    user_id = uuid.uuid4()

    resp = booking_client.post_booking(
        user_id=user_id,
        flight_id=flight_id,
        passenger_name="Test User",
        passenger_email="test@example.com",
        seat_count=10_000,
    )
    # Depending on implementation, this might validate seat_count or return conflict.
    assert resp.status_code in (400, 409)

