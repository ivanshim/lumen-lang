from bisect import insort_left, insort_right
from heapq import heapify, heappop, heappush, heapreplace, heappushpop
from pprint import pformat

a = [1, 5]
alias = a
insort_left(a, 3)
print(a, alias, a is alias)
def rebind(value):
    value = [99]
rebind(a)
print(a)
rows = [(1, 'a'), (2, 'b')]
insort_left(rows, (2, 'left'), key=lambda row: row[0])
insort_right(rows, (2, 'right'), key=lambda row: row[0])
print(pformat(rows))
h = [9, 1, 4, 2]
heapify(h)
heappush(h, 0)
print(heappop(h), heapreplace(h, 3), heappushpop(h, 2))
print([heappop(h) for i in range(len(h))])
