from itertools import groupby, pairwise
from pprint import pformat
print(pformat([(key, list(group)) for key, group in groupby('aabba')]))
saved = list(groupby('aabb'))
print([list(group) for key, group in saved])
print(pformat(list(pairwise([]))), pformat(list(pairwise([1]))))
print(pformat({'z': (3,), 'a': ['x', True, None]}))
