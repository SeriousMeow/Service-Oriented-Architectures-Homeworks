from __future__ import annotations

import time
import uuid

import pytest
import requests

from conftest import assert_error


def _toxiproxy_url() -> str:
    return "http://toxiproxy:8474"


def _proxy_enable(enabled: bool) -> None:
    r = requests.post(f"{_toxiproxy_url()}/proxies/flight", json={"enabled": enabled}, timeout=2)
    r.raise_for_status()


def _add_timeout_toxic(name: str = "timeout") -> None:
    r = requests.post(
        f"{_toxiproxy_url()}/proxies/flight/toxics",
        json={
            "name": name,
            "type": "timeout",
            "stream": "downstream",
            "toxicity": 1.0,
            "attributes": {"timeout": 1},
        },
        timeout=2,
    )
    r.raise_for_status()


def _remove_toxic(name: str) -> None:
    r = requests.delete(f"{_toxiproxy_url()}/proxies/flight/toxics/{name}", timeout=2)
    if r.status_code not in (200, 204, 404):
        r.raise_for_status()


@pytest.mark.req10
def test_circuit_breaker_opens_and_fast_fails(reset_and_seed: uuid.UUID, booking_client):
    flight_id = reset_and_seed

    _proxy_enable(True)
    _remove_toxic("timeout")
    _add_timeout_toxic("timeout")

    # Trigger failures until the breaker opens (defaults: threshold=5).
    for _ in range(6):
        r = booking_client.get_flight_by_id(flight_id)
        assert r.status_code == 503
        assert_error(r, 503, "FLIGHT_SERVICE_UNAVAILABLE")

    # Once OPEN, subsequent calls should fail fast (no upstream attempt).
    started = time.time()
    r2 = booking_client.get_flight_by_id(flight_id)
    elapsed = time.time() - started
    assert_error(r2, 503, "FLIGHT_SERVICE_UNAVAILABLE")
    assert elapsed < 0.2

    _remove_toxic("timeout")

    # Allow the breaker to transition OPEN->HALF_OPEN and close on a successful probe.
    time.sleep(0.35)
    r3 = booking_client.get_flight_by_id(flight_id)
    assert r3.status_code == 200, r3.text

