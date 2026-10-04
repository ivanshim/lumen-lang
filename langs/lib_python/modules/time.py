# Host primitives used by class bodies need non-private module bindings.
_host_clock = __clock

# Wall time comes from the system clock.
def time():
    return __clock(False)

# Stub: no waiting is performed by this small library.
def sleep(seconds):
    if seconds < 0:
        raise 'ValueError: sleep length must be non-negative'
    if type(seconds) != type(0) and type(seconds) != type(0.0):
        raise 'TypeError: a number is required'

# A steady clock counts elapsed seconds from a fixed origin.
def perf_counter():
    return __clock(True)

def monotonic():
    return __clock(True)

class _ClockInfo:
    def __init__(self, steady):
        self.implementation = 'clock_gettime(CLOCK_MONOTONIC)' if steady else 'clock_gettime(CLOCK_REALTIME)'
        self.monotonic = steady
        self.adjustable = not steady
        self.resolution = _host_clock(steady, True)

def get_clock_info(name):
    if name in ('monotonic', 'perf_counter'):
        return _ClockInfo(True)
    if name == 'time':
        return _ClockInfo(False)
    raise ValueError('unknown clock')

struct_time = _host_clock('struct_time')

def localtime(seconds=None):
    return _host_clock('localtime', seconds)

def gmtime(seconds=None):
    return _host_clock('gmtime', seconds)
