from __future__ import annotations

import threading
import time
import uuid

import pytest
import requests


def _toxiproxy_url() -> str:
    return "http://toxiproxy:8474"


def _proxy_enable(enabled: bool) -> None:
    r = requests.post(f"{_toxiproxy_url()}/proxies/flight", json={"enabled": enabled}, timeout=2)
    r.raise_for_status()


def _add_timeout_toxic(name: str = "timeout") -> None:
    r = requests.post(
        f"{_toxiproxy_url()}/proxies/flight/toxics",
        # toxiproxy:2.1.4 supports "timeout" toxic; "reset_peer" isn't available there.
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


@pytest.mark.req8
def test_booking_retries_transient_unavailable_and_recovers(reset_and_seed: uuid.UUID, booking_client):
    flight_id = reset_and_seed

    # Disable proxy briefly to force a transport failure, then re-enable so a retry can succeed.
    _proxy_enable(False)

    def _heal():
        time.sleep(0.05)
        _proxy_enable(True)

    t = threading.Thread(target=_heal, daemon=True)
    t.start()

    started = time.time()
    resp = booking_client.get_flight_by_id(flight_id)
    elapsed = time.time() - started

    assert resp.status_code == 200, resp.text
    # With one transient failure + retry backoff, we should not succeed instantly.
    assert elapsed >= 0.08
