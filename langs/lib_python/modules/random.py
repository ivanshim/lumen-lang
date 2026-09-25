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

def choices(population, weights=None, *, cum_weights=None, k=1):
    n = len(population)
    if cum_weights is not None:
        if weights is not None:
            raise 'TypeError: Cannot specify both weights and cumulative weights'
        cumulative = cum_weights
    elif weights is not None:
        cumulative = []
        total = 0
        for w in weights:
            total += w
            cumulative.append(total)
    else:
        cumulative = None
    result = []
    if cumulative is None:
        for _ in range(k):
            result.append(population[_next() % n])
    else:
        total = cumulative[-1]
        for _ in range(k):
            spot = random() * total
            at = 0
            while at < len(cumulative) - 1 and cumulative[at] <= spot:
                at += 1
            result.append(population[at])
    return result

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
