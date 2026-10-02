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

def sample(population, k):
    values = list(population)
    if k < 0 or k > len(values):
        raise ValueError("Sample larger than population or is negative")
    result = []
    for index in range(k):
        remaining = len(values) - index
        chosen = _next() % remaining
        result.append(values[chosen])
        values[chosen] = values[remaining - 1]
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


_unseeded_sequence = 0


class Random:
    """Mersenne Twister stream for independently seeded Random instances."""

    def __init__(self, seed=None):
        self.seed(seed)

    def seed(self, value=None):
        if value is None:
            try:
                import os
                value = int.from_bytes(os.urandom(16), 'big')
            except Exception:
                import time
                global _unseeded_sequence
                _unseeded_sequence += 1
                value = int(time.time() * 1000000000) + _unseeded_sequence
        value = abs(int(value))
        key = []
        while value:
            key.append(value & 0xffffffff)
            value >>= 32
        if not key:
            key = [0]
        mt = [0] * 624
        mt[0] = 19650218
        for i in range(1, 624):
            mt[i] = (1812433253 * (mt[i - 1] ^ (mt[i - 1] >> 30)) + i) & 0xffffffff
        i = 1
        j = 0
        for _ in range(max(624, len(key))):
            mt[i] = ((mt[i] ^ ((mt[i - 1] ^ (mt[i - 1] >> 30)) * 1664525)) + key[j] + j) & 0xffffffff
            i += 1
            j += 1
            if i >= 624:
                mt[0] = mt[623]
                i = 1
            if j >= len(key):
                j = 0
        for _ in range(623):
            mt[i] = ((mt[i] ^ ((mt[i - 1] ^ (mt[i - 1] >> 30)) * 1566083941)) - i) & 0xffffffff
            i += 1
            if i >= 624:
                mt[0] = mt[623]
                i = 1
        mt[0] = 0x80000000
        self._mt = mt
        self._index = 624

    def _word(self):
        if self._index >= 624:
            mt = self._mt
            for i in range(624):
                y = (mt[i] & 0x80000000) | (mt[(i + 1) % 624] & 0x7fffffff)
                mt[i] = mt[(i + 397) % 624] ^ (y >> 1)
                if y & 1:
                    mt[i] ^= 0x9908b0df
            self._index = 0
        y = self._mt[self._index]
        self._index += 1
        y ^= y >> 11
        y ^= (y << 7) & 0x9d2c5680
        y ^= (y << 15) & 0xefc60000
        y ^= y >> 18
        return y & 0xffffffff

    def getrandbits(self, k):
        if k < 0:
            raise ValueError('number of bits must be non-negative')
        if k == 0:
            return 0
        result = 0
        shift = 0
        while k > 0:
            take = min(k, 32)
            result |= (self._word() >> (32 - take)) << shift
            shift += take
            k -= take
        return result

    def choice(self, sequence):
        n = len(sequence)
        if n == 0:
            raise IndexError('Cannot choose from an empty sequence')
        k = n.bit_length()
        index = self.getrandbits(k)
        while index >= n:
            index = self.getrandbits(k)
        return sequence[index]


class SystemRandom(Random):
    """Random source backed by operating-system entropy."""
    def seed(self, *args, **kwargs):
        pass

    def random(self):
        import os
        return (int.from_bytes(os.urandom(7)) >> 3) * (2 ** -53)

    def getrandbits(self, k):
        import operator
        import os
        k = operator.index(k)
        if k < 0:
            raise ValueError('number of bits must be non-negative')
        if k == 0:
            return 0
        numbytes = (k + 7) // 8
        return int.from_bytes(os.urandom(numbytes)) >> (numbytes * 8 - k)

    def randbytes(self, n):
        import os
        return os.urandom(n)

    def _randbelow(self, n):
        k = n.bit_length()
        r = self.getrandbits(k)
        while r >= n:
            r = self.getrandbits(k)
        return r

    def getstate(self, *args, **kwargs):
        raise NotImplementedError('System entropy source does not have state.')

    setstate = getstate
