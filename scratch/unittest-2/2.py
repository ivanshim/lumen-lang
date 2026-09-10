import unittest

case = unittest.TestCase()
with case.assertRaises(case.failureException) as lists:
    case.assertEqual([1, 2], [1, 3])
print(str(lists.exception).split('\n')[0])
with case.assertRaises(case.failureException) as lines:
    case.assertEqual('first\nsecond\n', 'first\nthird\n')
print(str(lines.exception).split('\n')[0])
