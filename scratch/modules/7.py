import unittest
import itertools
import operator

@unittest.skip('deliberate')
class Skipped(unittest.TestCase):
    def test_one(self):
        self.fail('this test must be skipped')

result = unittest.TestResult()
Skipped('test_one').run(result)
print(result.testsRun, len(result.skipped), len(result.failures))
print([list(row) for row in itertools.product([1, 2], [3])])
print([list(row) for row in itertools.permutations([1, 2], 2)])
print([list(row) for row in itertools.combinations([1, 2, 3], 2)])
print(list(itertools.accumulate([1, 2, 3])))
print(operator.and_(6, 3), operator.xor(-1, 3))

from copy import deepcopy
class Box:
    def __init__(self, value):
        self.value = value

original = Box(Box(1))
cloned = deepcopy(original)
cloned.value.value = 2
print(original.value.value, cloned.value.value)
