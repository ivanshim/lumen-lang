from operator import add; import functools; print(functools.reduce(add, [1, 2, 3]))
import itertools; print(list(itertools.islice(itertools.count(5), 3)))
