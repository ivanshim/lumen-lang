import unittest
from io import StringIO

class Example(unittest.TestCase):
    def test_equal(self):
        self.assertEqual(2 + 2, 4)

    def test_truth(self):
        self.assertTrue(True)

    def test_wrong(self):
        self.assertEqual(1, 2)

# The runner now writes diagnostics to its stream and reports real failures.
stream = StringIO()
program = unittest.main(exit=False, testRunner=unittest.TextTestRunner(stream=stream))
result = program.result
print(result.testsRun, len(result.failures), len(result.errors), result.wasSuccessful())
print('FAIL: test_wrong (__main__.Example.test_wrong)' in stream.getvalue())
print('Ran 3 tests in ' in stream.getvalue())
print('FAILED (failures=1)' in stream.getvalue())
