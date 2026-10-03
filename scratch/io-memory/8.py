import unittest, types, tempfile, os, sys
class Left(unittest.TestCase):
    def test_left(self): pass
    def test_hidden(self): pass
class Right(unittest.TestCase):
    def test_right(self): pass
class Diamond(Left, Right):
    test_hidden = None
    def test_own(self): pass
loader = unittest.TestLoader()
print('MRO', loader.getTestCaseNames(Diamond))
print('count', loader.loadTestsFromTestCase(Diamond).countTestCases())
module = types.ModuleType('generated')
module.Diamond = Diamond
def hook(loader, tests, pattern):
    print('module hook', pattern, tests.countTestCases())
    tests.addTests(loader.loadTestsFromTestCase(Right))
    return tests
module.load_tests = hook
print('generated', loader.loadTestsFromModule(module, pattern='*.py').countTestCases())
def bad_hook(loader, tests, pattern):
    raise ValueError('hook failed')
module.load_tests = bad_hook
result = loader.loadTestsFromModule(module).run(unittest.TestResult())
print('hook error', result.testsRun, len(result.errors), len(loader.errors))
with tempfile.TemporaryDirectory() as root:
    package = os.path.join(root, 'discovery_package')
    os.mkdir(package)
    with open(os.path.join(package, '__init__.py'), 'w') as file:
        file.write("import os\ncalls = []\ntoken = 'package'\ndef load_tests(loader, tests, pattern):\n    globals()['token'] = 'package updated'\n    calls.append((pattern, token))\n    tests.addTests(loader.discover(os.path.dirname(__file__), pattern))\n    return tests\n")
    with open(os.path.join(package, 'test_generated.py'), 'w') as file:
        file.write("import unittest\ntoken = 'module'\nclass Helper:\n    value = 42\nclass Case(unittest.TestCase):\n    def test_original(self):\n        self.assertEqual(token, 'module')\n        self.assertEqual(self.Helper().value, 42)\ndef load_tests(loader, tests, pattern):\n    def generated(self):\n        self.assertEqual(globals()['token'], 'module')\n    Case.test_generated = generated\n    Case.Helper = globals()['Helper']\n    return loader.loadTestsFromTestCase(Case)\n")
    loader = unittest.TestLoader()
    suite = loader.discover(root)
    count = suite.countTestCases()
    result = suite.run(unittest.TestResult())
    import discovery_package
    print('package hook', discovery_package.calls)
    print('package tests', count, result.testsRun, len(result.errors), len(result.failures), len(loader.errors))
    sys.path.remove(root)

import doctest
docs = types.ModuleType('generated_docs')
docs.__doc__ = ">>> 6 * 7\n42\n"
def doc_hook(loader, tests, pattern):
    tests.addTests(doctest.DocTestSuite(docs))
    return tests
docs.load_tests = doc_hook
suite = unittest.TestLoader().loadTestsFromModule(docs)
result = suite.run(unittest.TestResult())
print('generated doctest', suite.countTestCases(), result.testsRun, len(result.errors), len(result.failures))
