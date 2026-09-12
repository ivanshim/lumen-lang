# Assertions in discovered methods must execute and become failures.
import unittest
from io import StringIO

class AssertionsExecute(unittest.TestCase):
    def test_contains(self):
        self.assertIn(4, [1, 2, 3])
        print('unreachable membership')

    def test_equal(self):
        self.assertEqual(1, 2)
        print('unreachable equality')

    def test_success(self):
        self.assertIn(2, [1, 2, 3])
        print('passing body')

stream = StringIO()
result = unittest.TextTestRunner(stream=stream).run(unittest.TestLoader().loadTestsFromTestCase(AssertionsExecute))
print(result.testsRun, len(result.failures), len(result.errors), result.wasSuccessful())
print('FAILED (failures=2)' in stream.getvalue())
