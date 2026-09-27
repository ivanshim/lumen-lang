import math
import random
from decimal import Decimal
values = list(range(12))
random.seed(17)
random.shuffle(values)
assert sorted(values) == list(range(12))
assert values != list(range(12))
random.seed(29)
pair = [random.gauss(), random.gauss()]
random.seed(29)
assert pair == [random.gauss(), random.gauss()]
assert 2.0 <= random.uniform(2.0, 3.0) <= 3.0
assert math.copysign(1.0, -float('nan')) == -1.0
assert Decimal('0.1') != 0.1
class Faulty:
    @property
    def bad(self):
        raise ValueError('descriptor')
try:
    hasattr(Faulty(), 'bad')
except ValueError:
    pass
else:
    raise AssertionError('descriptor exception hidden')
print('random mutation, cached sampling and numerical protocols preserved')

assert math.hypot(float('inf'), float('nan')) == float('inf')
assert math.dist((float('inf'), 0.0), (0.0, float('nan'))) == float('inf')
class InvalidCoordinate:
    def __float__(self):
        raise ValueError('late coordinate')
for operation in (lambda: math.hypot(float('inf'), InvalidCoordinate()),
                  lambda: math.dist((float('inf'), InvalidCoordinate()), (0.0, 0.0))):
    try:
        operation()
    except ValueError as error:
        assert str(error) == 'late coordinate'
    else:
        raise AssertionError('late conversion skipped')
class IteratedTuple(tuple):
    def __iter__(self):
        return iter((3.0, 4.0))
assert math.dist(IteratedTuple((99.0, 99.0)), (0.0, 0.0)) == 5.0

assert math.fsum([1e100, 1.0, -1e100]) == 1.0
assert math.fsum([2.0**53, 1.0, 2.0**-100]) == 2.0**53 + 2.0
assert math.fsum([-0.0, 0.0]) == 0.0

coordinates = []
class ChangingCoordinate:
    def __float__(self):
        coordinates[1] = 99.0
        return 3.0
coordinates = [ChangingCoordinate(), 4.0]
assert math.dist(coordinates, (0.0, 0.0)) == 5.0
assert coordinates[1] == 99.0
class IteratedList(list):
    def __iter__(self):
        return iter((3.0, 4.0))
assert math.dist(IteratedList((99.0, 99.0)), (0.0, 0.0)) == 5.0
