from collections import deque
def roundrobin(*iterables):
    pending = deque(iter(it) for it in iterables)
    print('len', len(pending))
    while pending:
        task = pending.popleft()
        try:
            yield next(task)
        except StopIteration:
            continue
        pending.append(task)
print(list(roundrobin('abc', 'd', 'efgh')))
