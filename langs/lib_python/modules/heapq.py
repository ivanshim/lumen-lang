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

def _siftdown(heap, start, pos):
    item = heap[pos]
    while pos > start:
        parent = (pos - 1) // 2
        if not item < heap[parent]:
            break
        heap[pos] = heap[parent]
        pos = parent
    heap[pos] = item

def _siftup(heap, pos):
    end = len(heap)
    start = pos
    item = heap[pos]
    child = 2 * pos + 1
    while child < end:
        right = child + 1
        if right < end and not heap[child] < heap[right]:
            child = right
        heap[pos] = heap[child]
        pos = child
        child = 2 * pos + 1
    heap[pos] = item
    _siftdown(heap, start, pos)

def heapify(x):
    for i in range(len(x) // 2 - 1, -1, -1):
        _siftup(x, i)

def heappush(heap, item):
    heap[:] = [*heap, item]
    _siftdown(heap, 0, len(heap) - 1)

def heappop(heap):
    if len(heap) == 0:
        raise 'IndexError: index out of range'
    last = heap[len(heap) - 1]
    result = heap[0]
    heap[:] = heap[:len(heap) - 1]
    if len(heap):
        heap[0] = last
        _siftup(heap, 0)
    return result

def heapreplace(heap, item):
    if len(heap) == 0:
        raise 'IndexError: index out of range'
    result = heap[0]
    heap[0] = item
    _siftup(heap, 0)
    return result

def heappushpop(heap, item):
    if len(heap) and heap[0] < item:
        result = heap[0]
        heap[0] = item
        _siftup(heap, 0)
        return result
    return item
