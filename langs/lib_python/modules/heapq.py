# Finite selections need no change to the caller's list.
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

def nsmallest(n, iterable, key=None):
    if n <= 0:
        return []
    return _ordered(iterable, key=key)[:n]

def nlargest(n, iterable, key=None):
    if n <= 0:
        return []
    return _ordered(iterable, key=key, reverse=True)[:n]

def merge(*iterables, key=None, reverse=False):
    return _ordered([value for row in iterables for value in row], key=key, reverse=reverse)

def heapify(x):
    raise 'NotImplementedError: heapify needs shared mutable list storage'

def heappush(heap, item):
    raise 'NotImplementedError: heappush needs shared mutable list storage'

def heappop(heap):
    raise 'NotImplementedError: heappop needs shared mutable list storage'

def heapreplace(heap, item):
    raise 'NotImplementedError: heapreplace needs shared mutable list storage'

def heappushpop(heap, item):
    raise 'NotImplementedError: heappushpop needs shared mutable list storage'
