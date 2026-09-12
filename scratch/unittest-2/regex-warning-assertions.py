import unittest
import warnings

case = unittest.TestCase()
case.assertCountEqual([1, 2, 1], [2, 1, 1])
case.assertAlmostEqual(1.0, 1.0001, places=3)
case.assertRegex('abc', 'b.')
case.assertRegex('abbbc', '^ab+c$')
case.assertRegex('ac', '^ab*c$')
case.assertNotRegex('abc', '^b.')
case.assertNotRegex('a\nb', 'a.b')
case.assertRegex('a.b', 'a\\.b')
with case.assertWarns(warnings.UserWarning):
    warnings.warn('old spelling', warnings.UserWarning)
print('assertions passed')
