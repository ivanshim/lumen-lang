from itertools import groupby, pairwise
print([(k, list(g)) for k, g in groupby("aabbc")], list(pairwise([1, 2, 3])))
from functools import lru_cache
@lru_cache
def f(n):
    return n * 2
print(f(2), f(2), f.cache_info().hits)
