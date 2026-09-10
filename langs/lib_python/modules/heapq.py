# Finite selections need no change to the caller's list.
def nsmallest(n, iterable, key=None):
    if n <= 0:
        return []
    return sorted(iterable, key=key)[:n]

def nlargest(n, iterable, key=None):
    if n <= 0:
        return []
    return sorted(iterable, key=key, reverse=True)[:n]

def merge(*iterables, key=None, reverse=False):
    return sorted([value for row in iterables for value in row], key=key, reverse=reverse)

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
