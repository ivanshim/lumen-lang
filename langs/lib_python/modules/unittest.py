# A small runner. Subtests keep their surrounding test's result. Regex
# assertions seek a substring; they do not interpret regular expressions.
class AssertionError:
    def __init__(self, message):
        self.message = message

class SkipTest:
    def __init__(self, message):
        self.message = message

class _Context:
    def __enter__(self):
        return self

    def __exit__(self, kind, value, traceback):
        return False

class _Raises:
    def __init__(self, expected, phrase=None):
        self.expected = expected
        self.phrase = phrase
        self.exception = None

    def __enter__(self):
        return self

    def __exit__(self, kind, value, traceback):
        if kind is None:
            raise AssertionError('exception not raised')
        if not isinstance(value, self.expected):
            return False
        self.exception = value
        if self.phrase is not None and self.phrase not in getattr(value, 'message', str(value)):
            raise AssertionError('exception message does not contain ' + self.phrase)
        return True

class TestResult:
    def __init__(self):
        self.testsRun = 0
        self.failures = []
        self.errors = []
        self.skipped = []
        self.expectedFailures = []
        self.unexpectedSuccesses = []

    def wasSuccessful(self):
        return len(self.failures) == 0 and len(self.errors) == 0 and len(self.unexpectedSuccesses) == 0

class TestCase:
    _test_case = True
    failureException = AssertionError

    def __init__(self, methodName='runTest'):
        self._method = methodName

    def setUp(self):
        pass

    def tearDown(self):
        pass

    def fail(self, msg=None):
        if msg is None:
            msg = 'test failed'
        raise AssertionError(msg)

    def _check(self, condition, message, msg=None):
        if not condition:
            if msg is not None:
                message = str(msg)
            self.fail(message)

    def assertEqual(self, a, b, msg=None):
        self._check(a == b, str(a) + ' != ' + str(b), msg)

    def assertNotEqual(self, a, b, msg=None):
        self._check(a != b, str(a) + ' == ' + str(b), msg)

    def assertTrue(self, value, msg=None):
        self._check(not not value, str(value) + ' is not true', msg)

    def assertFalse(self, value, msg=None):
        self._check(not value, str(value) + ' is not false', msg)

    def assertIs(self, a, b, msg=None):
        self._check(a is b, str(a) + ' is not ' + str(b), msg)

    def assertIsNot(self, a, b, msg=None):
        self._check(a is not b, 'unexpected identity', msg)

    def assertIsNone(self, value, msg=None):
        self.assertIs(value, None, msg)

    def assertIsNotNone(self, value, msg=None):
        self.assertIsNot(value, None, msg)

    def assertIn(self, member, container, msg=None):
        self._check(member in container, str(member) + ' not found in ' + str(container), msg)

    def assertNotIn(self, member, container, msg=None):
        self._check(member not in container, str(member) + ' unexpectedly found', msg)

    def assertIsInstance(self, value, kind, msg=None):
        self._check(isinstance(value, kind), 'value is not an instance of the requested class', msg)

    def assertGreater(self, a, b, msg=None):
        self._check(a > b, str(a) + ' not greater than ' + str(b), msg)

    def assertLess(self, a, b, msg=None):
        self._check(a < b, str(a) + ' not less than ' + str(b), msg)

    def assertGreaterEqual(self, a, b, msg=None):
        self._check(a >= b, str(a) + ' not greater than or equal to ' + str(b), msg)

    def assertLessEqual(self, a, b, msg=None):
        self._check(a <= b, str(a) + ' not less than or equal to ' + str(b), msg)

    def assertAlmostEqual(self, a, b, places=7, msg=None, delta=None):
        difference = a - b
        if difference < 0:
            difference = -difference
        if delta is not None:
            close = difference <= delta
        else:
            close = difference < 0.5 * 10 ** (-places)
        self._check(a == b or close, str(a) + ' != ' + str(b) + ' within tolerance', msg)

    def assertCountEqual(self, first, second, msg=None):
        left = list(first)
        right = list(second)
        self.assertEqual(len(left), len(right), msg)
        for value in left:
            a = 0
            b = 0
            for item in left:
                if item == value:
                    a += 1
            for item in right:
                if item == value:
                    b += 1
            self.assertEqual(a, b, msg)

    def assertRaises(self, expected, *args, **keywords):
        if len(args) == 0:
            return _Raises(expected)
        caught = False
        try:
            args[0](*args[1:], **keywords)
        except expected:
            caught = True
        self._check(caught, 'exception not raised')

    def assertRaisesRegex(self, expected, phrase, *args, **keywords):
        if len(args) == 0:
            return _Raises(expected, phrase)
        caught = False
        try:
            args[0](*args[1:], **keywords)
        except expected as error:
            caught = True
            self.assertIn(phrase, getattr(error, 'message', str(error)))
        self._check(caught, 'exception not raised')

    def subTest(self, msg=None, **params):
        # Stub: the body shares the surrounding test's result.
        return _Context()

    def skipTest(self, reason):
        raise SkipTest(reason)

    def _run_test(self):
        self.setUp()
        try:
            getattr(self, self._method)()
        finally:
            self.tearDown()

    def run(self, result=None):
        if result is None:
            result = TestResult()
        result.testsRun += 1
        outcome = __call_outcome(self._run_test)
        if outcome[0]:
            return result
        error = outcome[1]
        entry = [self._method, getattr(error, 'message', outcome[2])]
        if isinstance(error, AssertionError):
            result.failures = result.failures + [entry]
        elif isinstance(error, SkipTest):
            result.skipped = result.skipped + [entry]
        elif isinstance(error, _Expected):
            result.expectedFailures = result.expectedFailures + [entry]
        elif isinstance(error, _Unexpected):
            result.unexpectedSuccesses = result.unexpectedSuccesses + [entry]
        else:
            result.errors = result.errors + [entry]
        return result

class _Skip:
    def __init__(self, reason):
        self.reason = reason

    def call(self, *args, **kwargs):
        raise SkipTest(self.reason)

    def decorate(self, function):
        return self.call

def skip(reason):
    return _Skip(reason).decorate

def _identity(function):
    return function

def skipIf(condition, reason):
    if condition:
        return skip(reason)
    return _identity

def skipUnless(condition, reason):
    return skipIf(not condition, reason)

class _ExpectedFailure:
    def __init__(self, function):
        self.function = function

    def call(self, *args, **kwargs):
        try:
            self.function(*args, **kwargs)
        except SkipTest:
            raise
        except:
            raise _Expected('expected failure')
        raise _Unexpected('unexpected success')

class _Expected:
    def __init__(self, message):
        self.message = message

class _Unexpected:
    def __init__(self, message):
        self.message = message

def expectedFailure(function):
    return _ExpectedFailure(function).call

class TestSuite:
    def __init__(self, tests=None):
        self.tests = []
        if tests is not None:
            self.tests = list(tests)

    def addTest(self, test):
        self.tests = self.tests + [test]

    def addTests(self, tests):
        for test in tests:
            self.addTest(test)

    def run(self, result):
        for test in self.tests:
            test.run(result)
        return result

class TestLoader:
    def getTestCaseNames(self, cls):
        return [name for name in __class_methods(cls) if name[:4] == 'test']

    def loadTestsFromTestCase(self, cls):
        return TestSuite([cls(name) for name in self.getTestCaseNames(cls)])

def main(module=None, exit=True, verbosity=1):
    names = __program_namespace()
    result = TestResult()
    loader = TestLoader()
    for name in list(names):
        cls = names[name]
        if not getattr(cls, '_test_case', False):
            continue
        before = len(result.failures)
        errors = len(result.errors)
        loader.loadTestsFromTestCase(cls).run(result)
        for failure in result.failures[before:]:
            print('FAIL: ' + failure[0] + ' (' + name + ')')
            print('AssertionError: ' + failure[1])
        for error in result.errors[errors:]:
            print('ERROR: ' + error[0] + ' (' + name + ')')
            print(error[1])
    print('Ran ' + str(result.testsRun) + ' tests')
    if result.wasSuccessful():
        print('OK')
    else:
        print('FAILED (failures=' + str(len(result.failures)) + ', errors=' + str(len(result.errors)) + ')')
    # The scratch runner asks for the summary on stdout and a normal exit.
    return result
