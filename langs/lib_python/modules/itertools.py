# Iterator algorithms following CPython v3.14.8 / 8e6e75d9102e, Modules/itertoolsmodule.c
# and the equivalent Python recipes in Doc/library/itertools.rst; PSF License.
# CPython provides no Lib/itertools.py: these classes retain iterator state.
import operator as _operator
import sys as _sys

_unspecified = object()
_product_step = __itertools_product_step

def _rebuild_iterator(cls, state):
    result = object.__new__(cls)
    for key, value in state.items():
        setattr(result, key, value)
    return result

class _Iterator:
    _keyword_constructor = False
    def __new__(cls, *args, **kwargs):
        result = object.__new__(cls)
        if cls._keyword_constructor or cls.__init__ is _Iterator.__init__:
            cls._initialize(result, *args, **kwargs)
        else:
            cls._initialize(result, *args)
        return result
    def __init__(self, *args, **kwargs):
        pass
    def __reduce__(self):
        return (_rebuild_iterator, (type(self), {key: value for key, value in self.__dict__.items()}))
    def __iter__(self):
        return self

class count(_Iterator):
    _keyword_constructor = True
    def _initialize(self, start=0, step=1):
        for value in (start, step):
            kind = type(value)
            if not issubclass(kind, (int, float, complex)) and not any(
                name in base.__dict__
                for base in kind.__mro__
                for name in ('__index__', '__int__', '__float__')
            ):
                raise TypeError('a number is required')
        self.current = start
        self.step = step
    def __next__(self):
        result = self.current
        self.current = result + self.step
        return result
    def __repr__(self):
        if type(self.step) is int and self.step == 1:
            return 'count(%r)' % (self.current,)
        return 'count(%r, %r)' % (self.current, self.step)
    def __reduce__(self):
        return (type(self), (self.current, self.step))

class repeat(_Iterator):
    _keyword_constructor = True
    def _initialize(self, object, times=_unspecified):
        self.value = object
        if times is not _unspecified and not isinstance(times, int) and not hasattr(type(times), '__index__'):
            raise TypeError("'%s' object cannot be interpreted as an integer" % type(times).__name__)
        self.remaining = None if times is _unspecified else max(0, _operator.index(times))
    __next__ = _product_step("repeat")
    def __length_hint__(self):
        if self.remaining is None:
            raise TypeError('len() of unsized object')
        return self.remaining
    def __repr__(self):
        if self.remaining is None:
            return 'repeat(%r)' % (self.value,)
        return 'repeat(%r, %r)' % (self.value, self.remaining)
    def __reduce__(self):
        return (type(self), (self.value,) if self.remaining is None else (self.value, self.remaining))

class chain(_Iterator):
    def _initialize(self, *iterables):
        self.sources = iter(iterables)
        self.current = None
    @classmethod
    def from_iterable(cls, iterable, /):
        result = cls()
        result.sources = iter(iterable)
        return result
    def __next__(self):
        while True:
            if self.current is None:
                source = next(self.sources)
                self.current = iter(source)
            try:
                return next(self.current)
            except StopIteration:
                self.current = None
    def __reduce__(self):
        state = (self.sources,) if self.current is None else (self.sources, self.current)
        return (type(self), (), state)
    def __setstate__(self, state):
        self.sources = state[0]
        self.current = state[1] if len(state) > 1 else None

class islice(_Iterator):
    def _initialize(self, iterable, *args):
        if len(args) < 1 or len(args) > 3:
            raise TypeError('islice expected 2 to 4 arguments')
        start, stop, step = 0, args[0], 1
        if len(args) >= 2:
            start, stop = args[0], args[1]
        if len(args) == 3:
            step = args[2]
        try:
            start = 0 if start is None else _operator.index(start)
            stop = None if stop is None else _operator.index(stop)
            step = 1 if step is None else _operator.index(step)
        except TypeError:
            raise ValueError('Indices for islice() must be None or an integer: 0 <= x <= sys.maxsize.')
        if stop is not None and (stop < 0 or stop > _sys.maxsize):
            raise ValueError('Stop argument for islice() must be None or an integer: 0 <= x <= sys.maxsize.')
        if start < 0 or start > _sys.maxsize:
            raise ValueError('Indices for islice() must be None or an integer: 0 <= x <= sys.maxsize.')
        if step <= 0 or step > _sys.maxsize:
            raise ValueError('Step for islice() must be a positive integer or None.')
        self.source = iter(iterable)
        self.target, self.stop, self.step, self.position = start, stop, step, 0
        self.consume = start if stop is None else max(start, stop)
    __next__ = _product_step("islice")

class cycle(_Iterator):
    def _initialize(self, iterable, /):
        self.source = iter(iterable)
        self.saved = []
        self.position = 0
    def __next__(self):
        if self.source is not None:
            try:
                value = next(self.source)
            except StopIteration:
                self.source = None
            else:
                self.saved.append(value)
                return value
        if not self.saved:
            raise StopIteration
        value = self.saved[self.position]
        self.position = (self.position + 1) % len(self.saved)
        return value

class accumulate(_Iterator):
    _keyword_constructor = True
    def _initialize(self, iterable, func=None, *, initial=None):
        self.source = iter(iterable)
        self.function = func
        self.total = initial
        self.first = True
    def __next__(self):
        if self.first:
            self.first = False
            if self.total is not None:
                return self.total
            self.total = next(self.source)
            return self.total
        item = next(self.source)
        self.total = self.total + item if self.function is None else self.function(self.total, item)
        return self.total

class batched(_Iterator):
    _keyword_constructor = True
    def _initialize(self, iterable, n, /, *, strict=False):
        n = _operator.index(n)
        if n < 1:
            raise ValueError('n must be at least one')
        self.source = iter(iterable)
        self.n, self.strict = n, bool(strict)
        self.done = False
    def __next__(self):
        if self.done:
            raise StopIteration
        items = []
        for _ in range(self.n):
            try:
                items.append(next(self.source))
            except StopIteration:
                self.done = True
                break
        if not items:
            raise StopIteration
        if self.strict and len(items) != self.n:
            raise ValueError('batched(): incomplete batch')
        return tuple(items)

class starmap(_Iterator):
    def _initialize(self, function, iterable, /):
        self.function, self.source = function, iter(iterable)
    def __next__(self):
        args = tuple(next(self.source))
        return self.function(*args)

class compress(_Iterator):
    _keyword_constructor = True
    def _initialize(self, data, selectors):
        self.data, self.selectors = iter(data), iter(selectors)
    def __next__(self):
        while True:
            value = next(self.data)
            if next(self.selectors):
                return value

class filterfalse(_Iterator):
    def _initialize(self, function, iterable, /):
        self.function, self.source = function, iter(iterable)
    def __next__(self):
        while True:
            value = next(self.source)
            if not (bool(value) if self.function is None else self.function(value)):
                return value

class takewhile(_Iterator):
    def _initialize(self, predicate, iterable, /):
        self.predicate, self.source, self.done = predicate, iter(iterable), False
    def __next__(self):
        if self.done:
            raise StopIteration
        value = next(self.source)
        if self.predicate(value):
            return value
        self.done = True
        raise StopIteration

class dropwhile(_Iterator):
    def _initialize(self, predicate, iterable, /):
        self.predicate, self.source, self.dropping = predicate, iter(iterable), True
    def __next__(self):
        while True:
            value = next(self.source)
            if not self.dropping or not self.predicate(value):
                self.dropping = False
                return value

class zip_longest(_Iterator):
    def _initialize(self, *iterables, fillvalue=None):
        self.sources = [iter(it) for it in iterables]
        self.active = len(self.sources)
        self.fillvalue = fillvalue
    __next__ = _product_step("zip_longest")

class pairwise(_Iterator):
    def _initialize(self, iterable, /):
        self.source = iter(iterable)
        self.started = False
        self.previous = None
    def __next__(self):
        if not self.started:
            self.previous = next(self.source)
            self.started = True
        previous = self.previous
        value = next(self.source)
        self.previous = value
        return (previous, value)

class product(_Iterator):
    _keyword_constructor = True
    def _initialize(self, *iterables, repeat=1):
        repeat = _operator.index(repeat)
        if repeat < 0:
            raise ValueError('repeat argument cannot be negative')
        self.pools = [] if repeat == 0 else [tuple(it) for it in iterables] * repeat
        self.indices = [0] * len(self.pools)
        self.first = True
        self.done = any(not pool for pool in self.pools)
    __next__ = _product_step("product")

class combinations(_Iterator):
    _keyword_constructor = True
    def _initialize(self, iterable, r):
        self.pool = tuple(iterable)
        self.r = _operator.index(r)
        if self.r < 0:
            raise ValueError('r must be non-negative')
        self.indices = list(range(self.r))
        self.first = True
        self.done = self.r > len(self.pool)
    __next__ = _product_step("combinations")

class combinations_with_replacement(_Iterator):
    _keyword_constructor = True
    def _initialize(self, iterable, r):
        self.pool = tuple(iterable)
        self.r = _operator.index(r)
        if self.r < 0:
            raise ValueError('r must be non-negative')
        self.indices = [0] * self.r
        self.first = True
        self.done = not self.pool and self.r > 0
    def __next__(self):
        if self.done:
            raise StopIteration
        if self.first:
            self.first = False
        else:
            i = self.r - 1
            while i >= 0 and self.indices[i] == len(self.pool) - 1:
                i -= 1
            if i < 0:
                self.done = True
                raise StopIteration
            value = self.indices[i] + 1
            for j in range(i, self.r):
                self.indices[j] = value
        return tuple(self.pool[i] for i in self.indices)

class permutations(_Iterator):
    _keyword_constructor = True
    def _initialize(self, iterable, r=None):
        self.pool = tuple(iterable)
        n = len(self.pool)
        self.r = n if r is None else _operator.index(r)
        if self.r < 0:
            raise ValueError('r must be non-negative')
        self.indices = list(range(n))
        self.cycles = list(range(n, n - self.r, -1))
        self.first = True
        self.done = self.r > n
    def __next__(self):
        if self.done:
            raise StopIteration
        if self.first:
            self.first = False
            return tuple(self.pool[i] for i in self.indices[:self.r])
        n = len(self.pool)
        for i in range(self.r - 1, -1, -1):
            self.cycles[i] -= 1
            if self.cycles[i] == 0:
                self.indices[i:] = self.indices[i+1:] + self.indices[i:i+1]
                self.cycles[i] = n - i
            else:
                j = self.cycles[i]
                self.indices[i], self.indices[-j] = self.indices[-j], self.indices[i]
                return tuple(self.pool[k] for k in self.indices[:self.r])
        self.done = True
        raise StopIteration

class _Group(_Iterator):
    def _initialize(self, owner, key, generation):
        self.owner, self.key, self.generation = owner, key, generation
    def __next__(self):
        owner = self.owner
        if self.generation != owner.generation or not owner._peek():
            raise StopIteration
        if self.key is not owner.current_key and self.key != owner.current_key:
            raise StopIteration
        # Comparing user keys may consume the current item recursively.
        if not owner.ready:
            raise StopIteration
        value = owner.current
        owner.ready = False
        return value

class groupby(_Iterator):
    _keyword_constructor = True
    def _initialize(self, iterable, key=None):
        self.source, self.key = iter(iterable), key
        self.generation = 0
        self.ready, self.started, self.done = False, False, False
        self.target = None
    def _peek(self):
        if self.done:
            return False
        if not self.ready:
            try:
                value = next(self.source)
            except StopIteration:
                self.done = True
                return False
            self.current = value
            self.current_key = value if self.key is None else self.key(value)
            self.ready = True
        return True
    def __next__(self):
        self.generation += 1
        if self.started:
            while self._peek() and (self.target is self.current_key or self.target == self.current_key):
                self.ready = False
        if not self._peek():
            raise StopIteration
        self.target = self.current_key
        self.started = True
        return (self.target, _Group(self, self.target, self.generation))

class _TeeData:
    def __init__(self, source):
        self.state = _product_step('tee_data', source)
    def __getstate__(self):
        return _product_step('tee_snapshot', self.state)
    def __setstate__(self, state):
        self.state = _product_step('tee_restore', state)

def _restore_tee(data, position):
    result = object.__new__(_tee)
    result.data, result.position = data, position
    _product_step('tee_register', result)
    return result

class _tee(_Iterator):
    def _initialize(self, iterable, /):
        source = iter(iterable)
        if isinstance(source, _tee):
            self.data, self.position = source.data, source.position
        else:
            self.data = _TeeData(source)
            self.position = 0
        _product_step('tee_register', self)
    __next__ = _product_step('tee')
    def __copy__(self):
        return _tee(self)
    def __reduce__(self):
        return (_restore_tee, (self.data, self.position))

def tee(iterable, n=2, /):
    n = _operator.index(n)
    if n < 0:
        raise ValueError('n must be >= 0')
    source = iter(iterable)
    if n == 0:
        return ()
    if not isinstance(source, _tee):
        source = _tee(source)
    copies = [source.__copy__()]
    for _ in range(n - 1):
        copies.append(source.__copy__())
    return tuple(copies)
