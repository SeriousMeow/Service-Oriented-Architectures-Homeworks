import pytest
import redis


@pytest.mark.req9
def test_redis_sentinel_is_available_and_points_to_master():
    sentinel = redis.Redis(host="redis_sentinel", port=26379, decode_responses=False)
    assert sentinel.ping() is True
    master = sentinel.execute_command("SENTINEL", "get-master-addr-by-name", "mymaster")
    assert isinstance(master, (list, tuple))
    assert len(master) == 2
    assert master[0] == b"redis"
    assert master[1] == b"6379"

