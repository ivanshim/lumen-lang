import unittest
class Empty(unittest.TestCase):
    pass
class HiddenRun(unittest.TestCase):
    runTest = None
class DefaultRun(unittest.TestCase):
    def runTest(self):
        pass
loader = unittest.TestLoader()
for cls in (Empty, HiddenRun, DefaultRun):
    tests = loader.loadTestsFromTestCase(cls)
    result = unittest.TestResult()
    tests.run(result)
    print(cls.__name__, tests.countTestCases(), result.testsRun,
          len(result.failures), len(result.errors), len(result.skipped))
