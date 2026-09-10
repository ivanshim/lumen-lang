import unittest
from io import StringIO

class BasicTest(unittest.TestCase):
    def test_success(self):
        self.assertCountEqual([1, 2, 1], [1, 1, 2])
        self.assertAlmostEqual(1.0, 1.0001, places=3)
        self.assertNotAlmostEqual(1.0, 1.1, places=3)
        self.assertRegex('abc', 'bc')
        self.assertNotRegex('abc', 'de')
        self.assertHasAttr(self, '_method')

stream = StringIO()
result = unittest.TextTestRunner(stream=stream, verbosity=2).run(unittest.TestLoader().loadTestsFromTestCase(BasicTest))
print(result.testsRun, result.wasSuccessful())
print('test_success (__main__.BasicTest.test_success) ... ok' in stream.getvalue())
print('Ran 1 tests in ' in stream.getvalue())
