# Finite walks are gathered eagerly. Unbounded walks keep their rule;
# islice alone takes a finite portion without seeking an end.
class _Count:
    def __init__(self, start, step):
        self.start = start
        self.step = step

class _Repeat:
    def __init__(self, value):
        self.value = value

def count(start=0, step=1):
    return _Count(start, step)

def repeat(value, times=None):
    if times is None:
        return _Repeat(value)
    return [value for i in range(times)]

def islice(iterable, *bounds):
    start = 0
    step = 1
    if len(bounds) == 1:
        stop = bounds[0]
    elif len(bounds) == 2:
        start, stop = bounds
    elif len(bounds) == 3:
        start, stop, step = bounds
    else:
        raise 'TypeError: islice needs one to three bounds'
    if start < 0 or step <= 0 or (stop is not None and stop < 0):
        raise 'ValueError: invalid islice bounds'
    if isinstance(iterable, _Cycle):
        if stop is None:
            raise 'NotImplementedError: an unbounded cycle cannot be gathered'
        if len(iterable.values) == 0:
            return []
        result = [iterable.values[(iterable.position + i) % len(iterable.values)] for i in range(start, stop, step)]
        iterable.position = (iterable.position + stop) % len(iterable.values)
        return result
    if isinstance(iterable, _Count):
        if stop is None:
            raise 'NotImplementedError: an unbounded islice cannot be gathered'
        result = [iterable.start + i * iterable.step for i in range(start, stop, step)]
        iterable.start += stop * iterable.step
        return result
    if isinstance(iterable, _Repeat):
        if stop is None:
            raise 'NotImplementedError: an unbounded repeat cannot be gathered'
        return [iterable.value for i in range(start, stop, step)]
    result = []
    at = 0
    for item in iterable:
        if stop is not None and at >= stop:
            break
        if at >= start and (at - start) % step == 0:
            result.append(item)
        at += 1
    return result

def chain(*iterables):
    return [item for iterable in iterables for item in iterable]

def product(*iterables, repeat=1):
    result = [[]]
    for times in range(repeat):
        for iterable in iterables:
            result = [[*prefix, item] for prefix in result for item in iterable]
    return result

def permutations(iterable, r=None):
    values = list(iterable)
    if r is None:
        r = len(values)
    paths = [[]]
    for step in range(r):
        paths = [[*path, i] for path in paths for i in range(len(values)) if i not in path]
    return [[values[i] for i in path] for path in paths]

def combinations(iterable, r):
    values = list(iterable)
    paths = [[]]
    for step in range(r):
        fresh = []
        for path in paths:
            start = 0
            if len(path) != 0:
                start = path[len(path) - 1] + 1
            for i in range(start, len(values)):
                fresh.append([*path, i])
        paths = fresh
    return [[values[i] for i in path] for path in paths]

def zip_longest(*iterables, fillvalue=None):
    rows = [list(it) for it in iterables]
    length = 0
    for row in rows:
        if len(row) > length:
            length = len(row)
    return [[row[i] if i < len(row) else fillvalue for row in rows] for i in range(length)]

def accumulate(iterable, func=None, initial=None):
    result = []
    value = initial
    seen = initial is not None
    if seen:
        result.append(value)
    for item in iterable:
        if not seen:
            value = item
            seen = True
        elif func is None:
            value += item
        else:
            value = func(value, item)
        result.append(value)
    return result

# These finite walks share the eager gathering of this library's first
# routines. An unbounded source is refused before gathering begins.
def _finite(iterable):
    if isinstance(iterable, _Count) or isinstance(iterable, _Repeat) or isinstance(iterable, _Cycle):
        raise 'NotImplementedError: this operation needs a finite iterable'
    return list(iterable)

def takewhile(predicate, iterable):
    result = []
    for value in _finite(iterable):
        if not predicate(value):
            break
        result.append(value)
    return result

def dropwhile(predicate, iterable):
    result = []
    dropping = True
    for value in _finite(iterable):
        if dropping and predicate(value):
            continue
        dropping = False
        result.append(value)
    return result

def starmap(function, iterable):
    return [function(*args) for args in _finite(iterable)]

def compress(data, selectors):
    data, selectors = _finite(data), _finite(selectors)
    return [data[i] for i in range(len(data) if len(data) < len(selectors) else len(selectors)) if selectors[i]]

def filterfalse(predicate, iterable):
    if predicate is None:
        return [value for value in _finite(iterable) if not value]
    return [value for value in _finite(iterable) if not predicate(value)]

class _Cycle:
    def __init__(self, values):
        self.values = values
        self.position = 0

def cycle(iterable):
    return _Cycle(_finite(iterable))

class _Group:
    def __init__(self, owner, index):
        self.owner = owner
        self.index = index

    def __class_iter__(self):
        result = []
        while self.owner.active == self.index and self.owner._peek():
            if self.owner.current_key != self.owner.target:
                break
            result.append(self.owner.values[self.owner.position])
            self.owner._advance()
        return result

class _Grouped:
    def __init__(self, iterable, key):
        self.values = _finite(iterable)
        self.key = key
        self.position = 0
        self.active = -1
        self.ready = False
        self.pending = False
        self.started = False
        self.target = None
        self.current_key = None

    def _peek(self):
        if self.position >= len(self.values):
            return False
        if not self.ready:
            value = self.values[self.position]
            self.current_key = value if self.key is None else self.key(value)
            self.ready = True
        return True

    def _advance(self):
        self.position += 1
        self.ready = False

    def __class_iter__(self):
        return self

    def __has_index__(self, index):
        if self.pending:
            return True
        if self.started:
            while self._peek() and self.current_key == self.target:
                self._advance()
        if not self._peek():
            self.active += 1
            return False
        self.target = self.current_key
        self.active += 1
        self.pending = True
        return True

    def __getitem__(self, index):
        if not self.__has_index__(index):
            raise 'IndexError: group iterator exhausted'
        self.pending = False
        self.started = True
        return (self.target, _Group(self, self.active))

def groupby(iterable, key=None):
    return _Grouped(iterable, key)

def pairwise(iterable):
    values = _finite(iterable)
    return [(values[i], values[i + 1]) for i in range(len(values) - 1)]

def tee(iterable, n=2):
    raise 'NotImplementedError: tee needs tuple values and independent iterators'

def batched(iterable, n, strict=False):
    if n < 1:
        raise 'ValueError: n must be at least one'
    raise 'NotImplementedError: batched needs tuple values'

def combinations_with_replacement(iterable, r):
    if r < 0:
        raise 'ValueError: r must be non-negative'
    raise 'NotImplementedError: combinations_with_replacement needs tuple values'
