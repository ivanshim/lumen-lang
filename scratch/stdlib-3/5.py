import math
from itertools import takewhile, dropwhile, starmap, compress, filterfalse, cycle, islice
print(math.perm(5, 2), math.isqrt(100000000000000000000), math.dist([0, 0], [3, 4]))
print(math.sin(0), math.cos(0), math.log2(8), math.fmod(-7, 2))
print(math.nextafter(1, 2) > 1, math.ulp(1) > 0)
print(list(takewhile(lambda x: x < 3, [1, 2, 3, 1])))
print(list(dropwhile(lambda x: x < 3, [1, 2, 3, 1])))
print(list(starmap(lambda a, b: a + b, [[1, 2], [3, 4]])))
print(list(compress('ABC', [0, 1, 1])), list(filterfalse(None, [0, 1, False, 2])))
print(list(islice(cycle([1, 2]), 5)))
