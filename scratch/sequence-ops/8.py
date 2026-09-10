print([1] + [2, 3], (1,) + (2,), 'a' + 'b')
print([1] * -1, -2 * (1,), 'x' * 0, 3 * 'ab')
print((1, 2) * 2, 2 * (1, 2), 2 * [0])
try:
    print([1] + (2,))
except TypeError as e:
    print(str(e))
try:
    print((1,) + [2])
except TypeError as e:
    print(str(e))
print('a' + 1)
try:
    print([1] * 1.5)
except TypeError as e:
    print(str(e))
