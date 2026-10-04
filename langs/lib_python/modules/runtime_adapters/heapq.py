# Runtime adapter: the existing statistics module uses this private ordering helper.

def _ordered(iterable, key=None, reverse=False):
    values = []
    keys = []
    for item in iterable:
        wanted = item if key is None else key(item)
        at = len(values)
        while at > 0:
            before = wanted < keys[at - 1] if not reverse else keys[at - 1] < wanted
            if not before:
                break
            at -= 1
        values = [*values[:at], item, *values[at:]]
        keys = [*keys[:at], wanted, *keys[at:]]
    return values


# Native iterator wrappers are advanced through the iterator protocol.
from builtins import next as _next_item

def merge(*iterables, key=None, reverse=False):
    '''Merge multiple sorted inputs into a single sorted output.

    Similar to sorted(itertools.chain(*iterables)) but returns a generator,
    does not pull the data into memory all at once, and assumes that each of
    the input streams is already sorted (smallest to largest).

    >>> list(merge([1,3,5,7], [0,2,4,8], [5,10,15,20], [], [25]))
    [0, 1, 2, 3, 4, 5, 5, 7, 8, 10, 15, 20, 25]

    If *key* is not None, applies a key function to each element to determine
    its sort order.

    >>> list(merge(['dog', 'horse'], ['cat', 'fish', 'kangaroo'], key=len))
    ['dog', 'cat', 'fish', 'horse', 'kangaroo']

    '''

    h = []
    h_append = h.append

    if reverse:
        _heapify = heapify_max
        _heappop = heappop_max
        _heapreplace = heapreplace_max
        direction = -1
    else:
        _heapify = heapify
        _heappop = heappop
        _heapreplace = heapreplace
        direction = 1

    if key is None:
        for order, it in enumerate(map(iter, iterables)):
            try:
                next = it
                h_append([_next_item(next), order * direction, next])
            except StopIteration:
                pass
        _heapify(h)
        while len(h) > 1:
            try:
                while True:
                    value, order, next = s = h[0]
                    yield value
                    s[0] = _next_item(next)           # raises StopIteration when exhausted
                    _heapreplace(h, s)      # restore heap condition
            except StopIteration:
                _heappop(h)                 # remove empty iterator
        if h:
            # fast case when only a single iterator remains
            value, order, next = h[0]
            yield value
            while True:
                try:
                    yield _next_item(next)
                except StopIteration:
                    return
        return

    for order, it in enumerate(map(iter, iterables)):
        try:
            next = it
            value = _next_item(next)
            h_append([key(value), order * direction, value, next])
        except StopIteration:
            pass
    _heapify(h)
    while len(h) > 1:
        try:
            while True:
                key_value, order, value, next = s = h[0]
                yield value
                value = _next_item(next)
                s[0] = key(value)
                s[2] = value
                _heapreplace(h, s)
        except StopIteration:
            _heappop(h)
    if h:
        key_value, order, value, next = h[0]
        yield value
        while True:
            try:
                yield _next_item(next)
            except StopIteration:
                return
