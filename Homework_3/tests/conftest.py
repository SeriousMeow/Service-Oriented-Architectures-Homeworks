from __future__ import annotations

import os
import time
import uuid
from dataclasses import dataclass
from datetime import datetime, timezone
from typing import Any, Dict, Iterable, Optional, Tuple

import grpc
import psycopg
import pytest
import redis
import requests
from google.protobuf.timestamp_pb2 import Timestamp

from flight.v1 import flight_pb2, flight_pb2_grpc


def _env(name: str, default: Optional[str] = None) -> str:
    value = os.getenv(name, default)
    if value is None or value == "":
        raise RuntimeError(f"missing required env var: {name}")
    return value


def _wait_until(deadline_s: float, step_s: float, fn, what: str) -> None:
    last_exc: Optional[BaseException] = None
    while time.time() < deadline_s:
        try:
            if fn():
                return
        except BaseException as e:  # noqa: BLE001 - we want last error for debug
            last_exc = e
        time.sleep(step_s)
    if last_exc is None:
        raise RuntimeError(f"timeout waiting for {what}")
    raise RuntimeError(f"timeout waiting for {what}: {last_exc}") from last_exc


def _parse_rfc3339(ts: str) -> datetime:
    # Input examples from docker-compose: "2026-03-20T10:00:00+00"
    value = ts.replace("Z", "+00:00")
    if value.endswith("+00"):
        value = value + ":00"
    dt = datetime.fromisoformat(value)
    if dt.tzinfo is None:
        dt = dt.replace(tzinfo=timezone.utc)
    return dt.astimezone(timezone.utc)


def _grpc_timestamp(dt: datetime) -> Timestamp:
    ts = Timestamp()
    ts.FromDatetime(dt)
    return ts


@dataclass(frozen=True)
class SeedConfig:
    origin: str
    destination: str
    airline: str
    flight_number: str
    departure_time: datetime
    arrival_time: datetime
    total_seats: int
    available_seats: int
    price_currency: str
    price_minor_units: int


@pytest.fixture(scope="session")
def base_url() -> str:
    return _env("BASE_URL", "http://booking:8000").rstrip("/")


@pytest.fixture(scope="session")
def grpc_addr() -> str:
    return _env("GRPC_ADDR", "flight:50051")


@pytest.fixture(scope="session")
def service_api_key() -> str:
    return _env("SERVICE_API_KEY", "change-me")


@pytest.fixture(scope="session")
def redis_client() -> redis.Redis:
    host = _env("REDIS_HOST", "redis")
    port = int(_env("REDIS_PORT", "6379"))
    client = redis.Redis(host=host, port=port, decode_responses=False)
    _wait_until(time.time() + 60, 0.5, lambda: client.ping() is True, "redis ping")
    return client


def _pg_conninfo(prefix: str) -> str:
    host = _env(f"{prefix}_HOST")
    port = int(_env(f"{prefix}_PORT"))
    user = _env(f"{prefix}_USER")
    password = _env(f"{prefix}_PASSWORD")
    db = _env(f"{prefix}_DB")
    return f"postgresql://{user}:{password}@{host}:{port}/{db}"


def _redis_delete_patterns(client: redis.Redis, patterns: Iterable[str]) -> None:
    for pattern in patterns:
        for key in client.scan_iter(match=pattern, count=500):
            client.delete(key)


@pytest.fixture(scope="session")
def pg_flight_conninfo() -> str:
    return _pg_conninfo("POSTGRES_FLIGHT")


@pytest.fixture(scope="session")
def pg_booking_conninfo() -> str:
    return _pg_conninfo("POSTGRES_BOOKING")


@pytest.fixture(scope="session")
def grpc_channel(grpc_addr: str) -> grpc.Channel:
    channel = grpc.insecure_channel(grpc_addr)
    _wait_until(
        time.time() + 60,
        0.5,
        lambda: grpc.channel_ready_future(channel).result(timeout=1) is None or True,
        "grpc channel ready",
    )
    return channel


@pytest.fixture(scope="session")
def flight_stub(grpc_channel: grpc.Channel) -> flight_pb2_grpc.FlightServiceStub:
    return flight_pb2_grpc.FlightServiceStub(grpc_channel)


@pytest.fixture(scope="session")
def seed_cfg() -> SeedConfig:
    origin = _env("ORIGIN", "SVO").upper()
    destination = _env("DESTINATION", "LED").upper()
    airline = _env("AIRLINE", "TestAir")
    flight_number = _env("FLIGHT_NUMBER", "SU100")
    departure_time = _parse_rfc3339(_env("DEPARTURE_TIME", "2026-03-20T10:00:00+00"))
    arrival_time = _parse_rfc3339(_env("ARRIVAL_TIME", "2026-03-20T11:30:00+00"))
    total_seats = int(_env("TOTAL_SEATS", "100"))
    price_currency = _env("PRICE_CURRENCY", "RUB")
    price_minor_units = int(_env("PRICE_MINOR_UNITS", "500000"))
    return SeedConfig(
        origin=origin,
        destination=destination,
        airline=airline,
        flight_number=flight_number,
        departure_time=departure_time,
        arrival_time=arrival_time,
        total_seats=total_seats,
        available_seats=total_seats,
        price_currency=price_currency,
        price_minor_units=price_minor_units,
    )


@pytest.fixture()
def reset_and_seed(
    pg_flight_conninfo: str,
    pg_booking_conninfo: str,
    seed_cfg: SeedConfig,
    redis_client: redis.Redis,
) -> uuid.UUID:
    # Wait for both DBs.
    _wait_until(
        time.time() + 60,
        0.5,
        lambda: psycopg.connect(pg_flight_conninfo, connect_timeout=2).close() is None or True,
        "flight postgres connect",
    )
    _wait_until(
        time.time() + 60,
        0.5,
        lambda: psycopg.connect(pg_booking_conninfo, connect_timeout=2).close() is None or True,
        "booking postgres connect",
    )

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
                  %s, %s,
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
                    seed_cfg.total_seats,
                    seed_cfg.available_seats,
                    seed_cfg.price_currency,
                    seed_cfg.price_minor_units,
                ),
            )
            (flight_id,) = cur.fetchone()
        conn.commit()

    # Clear cache to avoid serving stale data between tests.
    _redis_delete_patterns(redis_client, ["flight:*", "search:*", "flight_search_keys:*"])

    with psycopg.connect(pg_booking_conninfo) as conn:
        with conn.cursor() as cur:
            cur.execute("TRUNCATE bookings RESTART IDENTITY CASCADE;")
        conn.commit()

    return flight_id


@pytest.fixture()
def auth_metadata(service_api_key: str) -> Tuple[Tuple[str, str], ...]:
    return (("x-service-api-key", service_api_key),)


def _json(resp: requests.Response) -> Dict[str, Any]:
    try:
        return resp.json()
    except Exception as e:  # noqa: BLE001
        raise AssertionError(f"expected json response, got status={resp.status_code} body={resp.text}") from e


@pytest.fixture(scope="session")
def http_session(base_url: str) -> requests.Session:
    s = requests.Session()

    def _ping() -> bool:
        try:
            r = s.get(f"{base_url}/flights", params={"origin": "SVO", "destination": "LED"}, timeout=2)
            # 503 can happen transiently while Flight is starting or breaker is warming up;
            # we still consider Booking "http ready" if it responds.
            return r.status_code in (200, 400, 503)
        except Exception:
            return False

    _wait_until(time.time() + 60, 0.5, _ping, "booking http ready")
    return s


@pytest.fixture()
def booking_client(http_session: requests.Session, base_url: str):
    class Client:
        def get_flights(self, origin: str, destination: str, date: Optional[str] = None) -> requests.Response:
            params = {"origin": origin, "destination": destination}
            if date is not None:
                params["date"] = date
            return http_session.get(f"{base_url}/flights", params=params, timeout=5)

        def get_flight_by_id(self, flight_id: uuid.UUID) -> requests.Response:
            return http_session.get(f"{base_url}/flights/{flight_id}", timeout=5)

        def post_booking(
            self,
            *,
            user_id: uuid.UUID,
            flight_id: uuid.UUID,
            passenger_name: str,
            passenger_email: str,
            seat_count: int,
        ) -> requests.Response:
            payload = {
                "user_id": str(user_id),
                "flight_id": str(flight_id),
                "passenger_name": passenger_name,
                "passenger_email": passenger_email,
                "seat_count": seat_count,
            }
            return http_session.post(f"{base_url}/bookings", json=payload, timeout=10)

        def get_booking_by_id(self, booking_id: uuid.UUID) -> requests.Response:
            return http_session.get(f"{base_url}/bookings/{booking_id}", timeout=5)

        def list_bookings(self, user_id: uuid.UUID) -> requests.Response:
            return http_session.get(f"{base_url}/bookings", params={"user_id": str(user_id)}, timeout=5)

        def cancel_booking(self, booking_id: uuid.UUID) -> requests.Response:
            return http_session.post(f"{base_url}/bookings/{booking_id}/cancel", timeout=10)

    return Client()


def _toxiproxy_url() -> str:
    return "http://toxiproxy:8474"


@pytest.fixture(autouse=True)
def reset_toxiproxy_and_breaker_between_tests(base_url: str) -> None:
    """
    The Booking service's circuit breaker is process-global. Since our test-suite
    runs against long-lived containers, we must ensure any injected upstream faults
    don't leak into unrelated tests.
    """
    try:
        requests.post(f"{_toxiproxy_url()}/proxies/flight", json={"enabled": True}, timeout=2)
        requests.delete(f"{_toxiproxy_url()}/proxies/flight/toxics/timeout", timeout=2)
    except Exception:
        # Best-effort cleanup; if toxiproxy is temporarily unavailable,
        # the normal readiness checks below will surface failures.
        pass

    deadline = time.time() + 10.0

    def _breaker_closed() -> bool:
        try:
            r = requests.get(
                f"{base_url}/flights",
                params={"origin": "SVO", "destination": "LED"},
                timeout=2,
            )
            return r.status_code != 503
        except Exception:
            return False

    _wait_until(deadline, 0.1, _breaker_closed, "circuit breaker closed")


@pytest.fixture(scope="session")
def grpc_helpers(seed_cfg: SeedConfig):
    class Helpers:
        def search_request(self) -> flight_pb2.SearchFlightsRequest:
            # Do not set departure_date for the default case (matches cache key no-date).
            return flight_pb2.SearchFlightsRequest(origin=seed_cfg.origin, destination=seed_cfg.destination)

        def search_request_with_date(self) -> flight_pb2.SearchFlightsRequest:
            t = _grpc_timestamp(seed_cfg.departure_time)
            return flight_pb2.SearchFlightsRequest(
                origin=seed_cfg.origin, destination=seed_cfg.destination, departure_date=t
            )

        def get_flight_request(self, flight_id: uuid.UUID) -> flight_pb2.GetFlightRequest:
            return flight_pb2.GetFlightRequest(flight_id=str(flight_id))

    return Helpers()


def assert_error(resp: requests.Response, status: int, code: str) -> Dict[str, Any]:
    assert resp.status_code == status, f"expected {status}, got {resp.status_code}: {resp.text}"
    data = _json(resp)
    assert data.get("code") == code, f"expected code={code}, got {data}"
    assert isinstance(data.get("message"), str)
    return data


def assert_booking(resp: requests.Response, expected_status: Optional[str] = None) -> Dict[str, Any]:
    data = _json(resp)
    for k in ("id", "user_id", "flight_id", "passenger_name", "passenger_email", "seat_count", "total_price", "status"):
        assert k in data, f"missing {k} in {data}"
    if expected_status is not None:
        assert data["status"] == expected_status
    return data


def assert_flight_item(item: Dict[str, Any], expected_id: Optional[uuid.UUID] = None) -> None:
    for k in (
        "id",
        "flight_number",
        "airline",
        "origin",
        "destination",
        "departure_time",
        "arrival_time",
        "total_seats",
        "available_seats",
        "price",
        "status",
    ):
        assert k in item, f"missing {k} in {item}"
    if expected_id is not None:
        assert item["id"] == str(expected_id)


# --- requirement PASS/FAIL summary plumbing ---

def pytest_configure(config: pytest.Config) -> None:
    config._req_outcomes = {  # type: ignore[attr-defined]
        "req5": [],
        "req6": [],
        "req7": [],
        "req8": [],
        "req9": [],
        "req10": [],
    }


@pytest.hookimpl(hookwrapper=True)
def pytest_runtest_makereport(item: pytest.Item, call: pytest.CallInfo):
    outcome = yield
    rep: pytest.TestReport = outcome.get_result()
    if rep.when != "call":
        return
    for m in ("req5", "req6", "req7", "req8", "req9", "req10"):
        if item.get_closest_marker(m) is not None:
            item.config._req_outcomes[m].append(rep.outcome)  # type: ignore[attr-defined]


def pytest_terminal_summary(terminalreporter: Any, exitstatus: int, config: pytest.Config) -> None:
    outcomes = getattr(config, "_req_outcomes", {})
    if not outcomes:
        return

    def status_for(marker: str) -> str:
        vals: Iterable[str] = outcomes.get(marker, [])
        vals = list(vals)
        if not vals:
            return "SKIP"
        if any(v != "passed" for v in vals):
            return "FAIL"
        return "PASS"

    terminalreporter.write_sep("=", "HW3 Requirements Summary (5-10)")
    terminalreporter.write_line(f"Req 5 (Tx integrity / concurrency): {status_for('req5')}")
    terminalreporter.write_line(f"Req 6 (gRPC auth UNAUTHENTICATED):  {status_for('req6')}")
    terminalreporter.write_line(f"Req 7 (Redis cache+TTL+inval):      {status_for('req7')}")
    terminalreporter.write_line(f"Req 8 (Booking->Flight retry):      {status_for('req8')}")
    terminalreporter.write_line(f"Req 9 (Redis HA sentinel):          {status_for('req9')}")
    terminalreporter.write_line(f"Req 10 (Circuit breaker):           {status_for('req10')}")

