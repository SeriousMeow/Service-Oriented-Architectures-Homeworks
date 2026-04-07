import grpc
import pytest

from flight.v1 import flight_pb2


@pytest.mark.req6
def test_grpc_missing_api_key_is_unauthenticated(flight_stub, grpc_helpers):
    req = grpc_helpers.search_request()
    with pytest.raises(grpc.RpcError) as e:
        flight_stub.SearchFlights(req, timeout=5)
    assert e.value.code() == grpc.StatusCode.UNAUTHENTICATED


@pytest.mark.req6
def test_grpc_invalid_api_key_is_unauthenticated(flight_stub, grpc_helpers):
    req = grpc_helpers.search_request()
    with pytest.raises(grpc.RpcError) as e:
        flight_stub.SearchFlights(req, metadata=(("x-service-api-key", "wrong-key"),), timeout=5)
    assert e.value.code() == grpc.StatusCode.UNAUTHENTICATED


@pytest.mark.req6
def test_grpc_valid_api_key_allows_calls(flight_stub, grpc_helpers, auth_metadata):
    req = grpc_helpers.search_request()
    resp = flight_stub.SearchFlights(req, metadata=auth_metadata, timeout=5)
    assert hasattr(resp, "flights")

