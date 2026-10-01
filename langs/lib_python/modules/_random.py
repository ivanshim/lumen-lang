# Mersenne Twister primitives corresponding to CPython Modules/_randommodule.c.
# PSF License.
_unseeded_sequence = 0
class Random:
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
        value = abs(value) if isinstance(value, int) else hash(value) & 0xffffffffffffffff
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
        word, self._index, changed = __math('mt19937', self._mt, self._index)
        if changed is not None:
            self._mt = changed
        return word

    def getrandbits(self, k):
        from operator import index
        k = index(k)
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

    def random(self):
        # The reference's own draw: twenty-seven figures of one word
        # and twenty-six of the next, scaled into [0, 1).
        high = self.getrandbits(32) >> 5
        low = self.getrandbits(32) >> 6
        return (high * 67108864.0 + low) * (1.0 / 9007199254740992.0)

    def getstate(self):
        return (*self._mt, self._index)

    def setstate(self, state):
        if not isinstance(state, tuple):
            raise TypeError('state vector must be a tuple')
        if len(state) != 625:
            raise ValueError('state vector is the wrong size')
        from operator import index
        words = [index(word) for word in state[:-1]]
        if any(word < 0 for word in words):
            raise OverflowError("can't convert negative value to unsigned int")
        if any(word > 0xffffffffffffffff for word in words):
            raise OverflowError("Python int too large to convert to C unsigned long")
        position = index(state[-1])
        if not 0 <= position <= 624:
            raise ValueError('invalid state')
        self._mt = [word & 0xffffffff for word in words]
        self._index = position
