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
