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
    rows = [list(row) for row in iterables]
    places = [0 for row in rows]
    heads = []
    for row in rows:
        value = None
        if len(row):
            value = row[0] if key is None else key(row[0])
        heads.append(value)
    result = []
    while True:
        chosen = -1
        active = 0
        for i in range(len(rows)):
            if places[i] >= len(rows[i]):
                continue
            active += 1
            earlier = chosen == -1
            if not earlier:
                earlier = heads[i] < heads[chosen] if not reverse else heads[chosen] < heads[i]
            if earlier:
                chosen = i
        if chosen == -1:
            break
        if active == 1:
            return [*result, *rows[chosen][places[chosen]:]]
        result.append(rows[chosen][places[chosen]])
        places[chosen] += 1
        if places[chosen] < len(rows[chosen]):
            item = rows[chosen][places[chosen]]
            heads[chosen] = item if key is None else key(item)
    return result

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
