import unittest
import warnings

case = unittest.TestCase()
case.assertCountEqual([1, 2, 1], [2, 1, 1])
case.assertAlmostEqual(1.0, 1.0001, places=3)
case.assertRegex('abc', 'b.')
with case.assertWarns(DeprecationWarning):
    warnings.warn('old spelling', DeprecationWarning)
print('assertions passed')
