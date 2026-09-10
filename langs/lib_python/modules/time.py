# Wall time comes from the system clock.
def time():
    return float(__clock())

# Stub: no waiting is performed by this small library.
def sleep(seconds):
    if seconds < 0:
        raise 'ValueError: sleep length must be non-negative'
    if type(seconds) != type(0) and type(seconds) != type(0.0):
        raise 'TypeError: a number is required'

# Stub: a wall clock must not pretend to be a steady clock.
def perf_counter():
    raise 'NotImplementedError: a steady clock is not available'

def monotonic():
    raise 'NotImplementedError: a steady clock is not available'
