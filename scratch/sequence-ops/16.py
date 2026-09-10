print([1] < [1, 0], () < (0,), [2] > [1, 9])
print([1] != (1,), [None] == [None])
n = float('nan')
print(n == n, n in [n], [n] == [n], (n,) == (n,))
print('é🙂é'.index('é', 1), 'é🙂é'.count('é'), list('é🙂'), tuple(reversed('é🙂')))
print('abc'.count('', 4), 'abc'.count('', 3), 'abc'.index('', 3))
try:
    print([1, 'x'] < [1, 2])
except TypeError as e:
    print(str(e))
try:
    print(None < None)
except TypeError as e:
    print(str(e))
