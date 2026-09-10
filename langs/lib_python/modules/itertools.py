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
                start = path[-1] + 1
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
