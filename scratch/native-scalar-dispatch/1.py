import math
left, right = int('9223372036854775807'), int('2')
assert left + right == 9223372036854775809
assert left * right == 18446744073709551614
assert -left // right == -4611686018427387904
assert -left % right == 1
x, y = float('2.5'), float('-0.5')
assert x + y == 2.0
assert x - y == 3.0
assert x * y == -1.25
assert math.isnan(x + float('nan'))
assert x * float('inf') == float('inf')
assert math.copysign(1.0, float('-0.0') * x) == -1.0
try:
    x + (1 << 10000)
except OverflowError:
    pass
else:
    raise AssertionError('huge integer conversion lost its overflow')
class Number(int):
    # Native subclasses must still dispatch their Python overrides.
    def __add__(self, other): return 'left'
    def __radd__(self, other): return 'right'
assert Number(1) + 2 == 'left'
assert 2 + Number(1) == 'right'
assert [1] + [2] == [1, 2]
assert 'a' < 'b'
assert 3 > 2
print('native arithmetic preserves overflow, special floats and subclass dispatch')
