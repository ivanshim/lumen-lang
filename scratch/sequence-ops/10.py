print([1, 2] in [[1, 2]], (1, 2) in [(1, 2)], [1] not in [(1,)])
print('bc' in 'abcd', '' in 'abc', 'ac' not in 'abcd')
print([1, 2, 1, 2].index(2, 2, 4), (1, 2, 1).index(1, -2))
print('banana'.index('an', 2, 6), 'aaaa'.count('aa'))
print([1, 1.0, True].count(1), (1, 2, 1).count(1), 'banana'.count('a', 2, 5))
try:
    print([1, 2].index(3))
except ValueError as e:
    print(str(e))
try:
    print((1, 2).index(3))
except ValueError as e:
    print(str(e))
try:
    print('abc'.index('z'))
except ValueError as e:
    print(str(e))
