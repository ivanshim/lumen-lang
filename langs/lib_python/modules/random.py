# A repeatable linear congruential generator, not the reference's stream.
_state = 1

def seed(a=None, version=2):
    global _state, _gauss_next
    _gauss_next = None
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
    distance = values.stop - values.start
    if values.step < 0:
        distance = -distance
    if distance <= 0:
        raise 'ValueError: empty range for randrange()'
    count = (distance - 1) // abs(values.step) + 1
    return values[_next() % count]

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
    for i in range(len(sequence) - 1, 0, -1):
        j = randrange(i + 1)
        saved = sequence[i]
        sequence[i] = sequence[j]
        sequence[j] = saved

def uniform(a, b):
    return a + (b - a) * random()


_gauss_next = None


def gauss(mu=0.0, sigma=1.0):
    import math
    global _gauss_next
    z = _gauss_next
    _gauss_next = None
    if z is None:
        angle = random() * math.tau
        radius = math.sqrt(-2.0 * math.log(1.0 - random()))
        z = math.cos(angle) * radius
        _gauss_next = math.sin(angle) * radius
    return mu + z * sigma



def getrandbits(k):
    if k < 0:
        raise 'ValueError: number of bits must be non-negative'
    result = 0
    for i in range(k):
        result = result * 2 + _next() % 2
    return result
