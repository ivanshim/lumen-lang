import unittest
import functools
import operator
import random
import math

case = unittest.TestCase()
case.assertRaises(case.failureException, case.fail, 'caught by call')
case.assertRaisesRegex(case.failureException, 'caught', case.fail, 'caught by call')
print('call assertions passed')
plus = functools.partial(operator.add, 3)
print(plus(4))
random.seed(17)
first = random.random()
random.seed(17)
print(first == random.random(), 0 <= first < 1)
print(math.isclose(math.log(math.e), 1), math.isinf(math.inf), math.isnan(math.nan))

class Outcomes(unittest.TestCase):
    def test_failure(self):
        self.fail('one failure')

    def test_error(self):
        missing_routine()

result = unittest.TestResult()
Outcomes('test_failure').run(result)
Outcomes('test_error').run(result)
print(result.testsRun, len(result.failures), len(result.errors))
import test.support
import test.support as helpers
print(test.support.verbose, helpers is test.support)
