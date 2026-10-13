from itertools import repeat
from operator import length_hint
item = object()
iterator = repeat(item, 3)
assert iter(iterator) is iterator
assert length_hint(iterator) == 3
assert next(iterator) is item
assert length_hint(iterator) == 2
assert list(iterator) == [item, item]
assert length_hint(iterator) == 0
try: next(iterator)
except StopIteration: pass
else: raise AssertionError('repeat did not exhaust')
assert list(repeat(item, -3)) == []
assert list(repeat(item, 0)) == []
assert sum(repeat(23, 1000)) == 23000
unbounded = repeat(item)
for _ in range(7):
    assert next(unbounded) is item
print('repeat preserves identity, remaining length and exhaustion')
