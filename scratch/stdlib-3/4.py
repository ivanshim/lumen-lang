import bisect, heapq, statistics, array, decimal, codecs, string, struct, pprint
from functools import lru_cache
@lru_cache
def twice(n):
    return n * 2
print(twice(2), twice(2), twice.cache_info().hits)
a = array.array('i', [1, 2])
a.append(3)
a.extend([4])
print(a.tolist(), a.itemsize, a.typecode)
print(bisect.bisect_left([1, 3, 5], 3), bisect.bisect_right([1, 3, 5], 3))
print(string.capwords(' ab  CD '))
print(codecs.lookup('utf-8').name)
decimal.getcontext().prec = 17
print(decimal.getcontext().prec)
