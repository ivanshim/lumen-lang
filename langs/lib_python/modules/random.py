# A repeatable linear congruential generator, not the reference's stream.
_state = 1

def seed(a=None, version=2):
    global _state
    if a is None:
        a = 1
    _state = int(a) % 2147483648
    if _state < 0:
        _state += 2147483648

def _next():
    global _state
    _state = (1103515245 * _state + 12345) % 2147483648
    return _state

def random():
    return _next() / 2147483648

def randrange(start, stop=None, step=1):
    if stop is None:
        start, stop = 0, start
    values = range(start, stop, step)
    if len(values) == 0:
        raise 'ValueError: empty range for randrange()'
    return values[_next() % len(values)]

def randint(a, b):
    return randrange(a, b + 1)

def choice(sequence):
    if len(sequence) == 0:
        raise 'IndexError: Cannot choose from an empty sequence'
    return sequence[_next() % len(sequence)]

def shuffle(sequence):
    # Ordinary arrays are copied on writes through a function argument;
    # returning a shuffled copy would conceal a missing in-place change.
    raise 'NotImplementedError: shuffle needs shared mutable sequence storage'

def getrandbits(k):
    if k < 0:
        raise 'ValueError: number of bits must be non-negative'
    result = 0
    for i in range(k):
        result = result * 2 + _next() % 2
    return result
