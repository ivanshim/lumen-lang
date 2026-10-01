# The Mersenne Twister core the reference implementation keeps in C
# (Modules/_randommodule.c), drawn here from the kernel's own stream
# (ext.builtin._random). The seeding, the drawing and the shape of the
# state are the reference's own: a seed becomes unsigned 32-bit words
# read from its low end, a draw takes 53 bits of two tempered words,
# and the state is the 624 kept words with the place among them.
# Copyright (c) 2001 Python Software Foundation; All Rights Reserved.
# The PSF license is kept in tests/python/LICENSE.

VERSION = 3

_WORDS = 624
_UNSIGNED_LONG = (1 << 64) - 1
_UNSIGNED_INT = (1 << 32) - 1


class Random:
    """Random number generator based on the Mersenne Twister core generator.
    """

    def _lent(self):
        # The mark the stream this generator draws from answers to.
        # Opened here rather than at construction: a subclass of this
        # class may write its own beginning and reach this stream's
        # workings without ever passing through this one's.
        held = getattr(self, '_stream', None)
        if held is None:
            held = __random('begin')
            self._stream = held
        return held

    def __init__(self, x=None):
        self.seed(x)

    def seed(self, a=None, /):
        """Initialize internal state from a seed.

        None or no argument seeds from the operating system's own source
        of disorder, or from the clock and the process mark when that
        source does not answer. An int seeds from all of its bits, its
        sign taken off first; anything else seeds from its hash.
        """
        if a is None:
            if not __random('entropy', self._lent()):
                # The worst disorder stands in: the wall clock and a
                # steady clock, each with its two words, around the
                # process mark.
                import os
                import time
                now = int(time.time() * 1000000000)
                since = int(time.monotonic() * 1000000000)
                words = [now & 0xffffffff, (now >> 32) & 0xffffffff,
                         os.getpid() & 0xffffffff,
                         since & 0xffffffff, (since >> 32) & 0xffffffff]
                mixed = 0
                for word in reversed(words):
                    mixed = (mixed << 32) | word
                __random('seed', self._lent(), mixed)
            return None
        if type(a) is int:
            number = a if a >= 0 else -a
        elif isinstance(a, int):
            # A subclass may spell __abs__ of its own and spell it
            # wrongly; the int class's own is asked, as the reference
            # asks it (issue 31478).
            number = int.__abs__(a)
        else:
            number = hash(a) & ((1 << 64) - 1)
        __random('seed', self._lent(), number)
        return None

    def random(self):
        """Get the next random number in the range [0.0, X < 1.0)."""
        held = getattr(self, '_stream', None)
        if held is None:
            held = self._lent()
        return __random('next', held)

    def getrandbits(self, k, /):
        """getrandbits(k) -> x.  Generates an int with k random bits."""
        if type(k) is not int:
            if getattr(type(k), '__index__', None) is None:
                raise TypeError("'{}' object cannot be interpreted as an integer".format(type(k).__name__))
            from operator import index
            k = index(k)
        if k < 0:
            raise ValueError('Cannot convert negative int to unsigned '
                             '64-bit integer')
        if k > _UNSIGNED_LONG:
            raise OverflowError('Python int too large to convert to '
                                'C uint64_t')
        held = getattr(self, '_stream', None)
        if held is None:
            held = self._lent()
        return __random('bits', held, k)

    def getstate(self):
        """getstate() -> tuple containing the current state."""
        return __random('state', self._lent())

    def setstate(self, state, /):
        """setstate(state) -> None.  Restores generator state."""
        if not isinstance(state, tuple):
            raise TypeError('state vector must be a tuple')
        if len(state) != _WORDS + 1:
            raise ValueError('state vector is the wrong size')
        words = []
        for place in range(_WORDS):
            given = state[place]
            if not isinstance(given, int):
                raise TypeError('an integer is required')
            if given < 0:
                raise OverflowError("can't convert negative value to "
                                    'unsigned int')
            if given > _UNSIGNED_LONG:
                raise OverflowError('Python int too large to convert to '
                                    'C unsigned long')
            # The unsigned long the words are read into holds 64 bits
            # here, and each word keeps only its lower 32.
            words.append(given & _UNSIGNED_INT)
        place = state[_WORDS]
        if not isinstance(place, int):
            raise TypeError('an integer is required')
        if place < -(1 << 63) or place > (1 << 63) - 1:
            raise OverflowError('Python int too large to convert to C long')
        if place < 0 or place > _WORDS:
            raise ValueError('invalid state')
        __random('restore', self._lent(), words + [place])

    def __reduce__(self):
        raise TypeError("cannot pickle '_random.Random' object")
