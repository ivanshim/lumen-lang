# Stand-in for CPython's _uuid extension (libuuid): version-1 UUIDs from the
# clock, a random 14-bit clock sequence and the first non-zero hardware address
# (a random multicast node when there is none), as libuuid does.
import os, time

has_uuid_generate_time_safe = 1

def _hardware_node():
    # libuuid reads the first interface with a non-zero hardware address;
    # only addresses that fit the 48-bit UUID node field are usable
    try:
        names = sorted(os.listdir("/sys/class/net"))
    except OSError:
        return None
    for name in names:
        try:
            with open("/sys/class/net/" + name + "/address") as f:
                node = int(f.read().strip().replace(":", ""), 16)
        except (OSError, ValueError):
            continue
        if 0 < node < (1 << 48):
            return node
    return None

_node = _hardware_node()
has_stable_extractable_node = _node is not None
_last = 0
_clock_seq = int.from_bytes(os.urandom(2), "big") & 0x3fff
if _node is None:
    _node = int.from_bytes(os.urandom(6), "big") | 0x010000000000

def generate_time_safe():
    global _last
    ticks = time.time_ns() // 100 + 0x01b21dd213814000
    if ticks <= _last:
        ticks = _last + 1
    _last = ticks
    value = (ticks & 0xffffffff) << 96
    value |= ((ticks >> 32) & 0xffff) << 80
    value |= (((ticks >> 48) & 0x0fff) | 0x1000) << 64
    value |= (0x8000 | _clock_seq) << 48
    value |= _node
    # -1: uniqueness across processes is not guaranteed without uuidd
    return value.to_bytes(16, "big"), -1
