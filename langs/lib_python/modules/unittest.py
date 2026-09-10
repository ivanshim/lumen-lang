# Assertions and the test lifecycle live beside the modules they serve.
# In the absence of a regular-expression module, phrases are literal.
class AssertionError:
    def __init__(self, message):
        self.message = str(message)
        self.args = (message,)

    def __str__(self):
        return self.message

class SkipTest:
    def __init__(self, message):
        self.message = str(message)
        self.args = (message,)

    def __str__(self):
        return self.message

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
            raise AssertionError(__class_name(self.expected) + ' not raised')
        if not isinstance(value, self.expected):
            return False
        self.exception = value
        if self.phrase is not None and not _matches(self.phrase, _message(value)):
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
    maxDiff = 640
    longMessage = True

    def __init__(self, methodName='runTest'):
        self._method = methodName
        self._cleanups = []
        self._result = None

    def setUpClass(cls):
        pass

    def tearDownClass(cls):
        pass

    def setUp(self):
        pass

    def tearDown(self):
        pass

    def fail(self, msg=None):
        raise self.failureException(msg)

    def _check(self, condition, message, msg=None):
        if not condition:
            if msg is not None:
                if self.longMessage:
                    message += ' : ' + str(msg)
                else:
                    message = str(msg)
            self.fail(message)

    def assertEqual(self, a, b, msg=None):
        if a == b:
            return None
        left = _representation(a)
        right = _representation(b)
        message = left + ' != ' + right
        if left[:1] == '[' and right[:1] == '[':
            message = 'Lists differ: ' + message
            limit = len(a)
            if len(b) < limit:
                limit = len(b)
            for index in range(limit):
                if a[index] != b[index]:
                    message += '\n\nFirst differing element ' + str(index) + ':\n' + _representation(a[index]) + '\n' + _representation(b[index])
                    break
            diff = '\n\n- ' + left + '\n+ ' + right
            message = self._with_diff(message, diff)
        elif type(a) == type('') and ('\n' in a or '\n' in b):
            message = self._with_diff(message, '\n' + _line_diff(a, b))
        elif left[:1] == '{' and right[:1] == '{':
            message = self._with_diff(message, '\n- ' + left + '\n+ ' + right)
        self._check(False, message, msg)

    def _with_diff(self, message, diff):
        if self.maxDiff is not None and len(diff) > self.maxDiff:
            return message + '\nDiff is ' + str(len(diff)) + ' characters long. Set self.maxDiff to None to see it.'
        return message + diff

    def id(self):
        return getattr(self, '_test_module', '__main__') + '.' + __class_name(self) + '.' + self._method

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

    def assertAlmostEqual(self, a, b, places=None, msg=None, delta=None):
        if delta is not None and places is not None:
            raise 'TypeError: specify delta or places not both'
        if places is None:
            places = 7
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
        with _Raises(expected):
            args[0](*args[1:], **keywords)

    def assertRaisesRegex(self, expected, phrase, *args, **keywords):
        if len(args) == 0:
            return _Raises(expected, phrase)
        with _Raises(expected, phrase):
            args[0](*args[1:], **keywords)

    def subTest(self, msg=None, **params):
        return _SubTest(self, msg, params)

    def skipTest(self, reason):
        raise SkipTest(reason)

    def addCleanup(self, function, *args, **kwargs):
        self._cleanups = [*self._cleanups, [function, args, kwargs]]

    def doCleanups(self):
        successful = True
        while len(self._cleanups) > 0:
            cleanup = self._cleanups[len(self._cleanups) - 1]
            self._cleanups = self._cleanups[:-1]
            outcome = __call_outcome(_Call(cleanup[0], cleanup[1], cleanup[2]).invoke)
            if not outcome[0]:
                successful = False
                if self._result is None:
                    raise outcome[1]
                self._result.errors = [*self._result.errors, [self._method, _message(outcome[1], outcome[2]), self.id()]]
        return successful

    def shortDescription(self):
        return None

    def assertIsSubclass(self, cls, superclass, msg=None):
        raise 'NotImplementedError: subclass assertions need class ancestry inspection'

    def assertNotIsInstance(self, value, kind, msg=None):
        self._check(not isinstance(value, kind), _representation(value) + ' is an instance of the requested class', msg)

    def assertNotAlmostEqual(self, a, b, places=None, msg=None, delta=None):
        if delta is not None and places is not None:
            raise 'TypeError: specify delta or places not both'
        if places is None:
            places = 7
        difference = a - b
        if difference < 0:
            difference = -difference
        if delta is not None:
            close = difference <= delta
        else:
            close = difference < 0.5 * 10 ** (-places)
        self._check(not (a == b or close), _representation(a) + ' == ' + _representation(b) + ' within tolerance', msg)

    def assertSequenceEqual(self, first, second, msg=None, seq_type=None):
        if seq_type is not None:
            self.assertIsInstance(first, seq_type, msg)
            self.assertIsInstance(second, seq_type, msg)
        left = list(first)
        right = list(second)
        self.assertEqual(left, right, msg)

    def assertListEqual(self, first, second, msg=None):
        self.assertSequenceEqual(first, second, msg)

    def assertTupleEqual(self, first, second, msg=None):
        self.assertSequenceEqual(first, second, msg)

    def assertDictEqual(self, first, second, msg=None):
        self.assertEqual(first, second, msg)

    def assertSetEqual(self, first, second, msg=None):
        for item in first:
            self.assertIn(item, second, msg)
        for item in second:
            self.assertIn(item, first, msg)

    def assertMultiLineEqual(self, first, second, msg=None):
        self.assertEqual(first, second, msg)

    def assertRegex(self, text, pattern, msg=None):
        self._check(_matches(pattern, text), "Regex didn't match: " + _representation(pattern) + ' not found in ' + _representation(text), msg)

    def assertNotRegex(self, text, pattern, msg=None):
        self._check(not _matches(pattern, text), 'Regex matched: ' + _representation(pattern) + ' matches ' + _representation(text), msg)

    def assertWarns(self, expected, *args, **kwargs):
        context = _Warns(expected)
        if len(args) == 0:
            return context
        with context:
            args[0](*args[1:], **kwargs)

    def assertWarnsRegex(self, expected, pattern, *args, **kwargs):
        context = _Warns(expected, pattern)
        if len(args) == 0:
            return context
        with context:
            args[0](*args[1:], **kwargs)

    def assertLogs(self, logger=None, level=None):
        raise 'NotImplementedError: log capture is not supported'

    def assertHasAttr(self, obj, name, msg=None):
        marker = _Context()
        self._check(getattr(obj, name, marker) is not marker, 'attribute ' + _representation(name) + ' is missing', msg)

    def assertNotHasAttr(self, obj, name, msg=None):
        marker = _Context()
        self._check(getattr(obj, name, marker) is marker, 'attribute ' + _representation(name) + ' is present', msg)

    def assertStartsWith(self, text, prefix, msg=None):
        self._check(text[:len(prefix)] == prefix, _representation(text) + ' does not start with ' + _representation(prefix), msg)

    def assertNotStartsWith(self, text, prefix, msg=None):
        self._check(text[:len(prefix)] != prefix, _representation(text) + ' starts with ' + _representation(prefix), msg)

    def assertEndsWith(self, text, suffix, msg=None):
        self._check(suffix == '' or text[-len(suffix):] == suffix, _representation(text) + ' does not end with ' + _representation(suffix), msg)

    def _run_test(self):
        if getattr(self, '__unittest_skip__', False):
            self.skipTest(getattr(self, '__unittest_skip_why__', 'skipped'))
        self.setUp()
        try:
            getattr(self, self._method)()
        finally:
            self.tearDown()

    def run(self, result=None):
        if result is None:
            result = TestResult()
        self._result = result
        result.testsRun += 1
        outcome = __call_outcome(self._run_test)
        self.doCleanups()
        if outcome[0]:
            return result
        error = outcome[1]
        entry = [self._method, getattr(error, 'message', outcome[2]), self.id()]
        if isinstance(error, self.failureException):
            result.failures = [*result.failures, entry]
        elif isinstance(error, SkipTest):
            result.skipped = [*result.skipped, entry]
        elif isinstance(error, _Expected):
            result.expectedFailures = [*result.expectedFailures, entry]
        elif isinstance(error, _Unexpected):
            result.unexpectedSuccesses = [*result.unexpectedSuccesses, entry]
        else:
            result.errors = [*result.errors, entry]
        return result

class _Skip:
    def __init__(self, reason):
        self.reason = reason

    def call(self, *args, **kwargs):
        raise SkipTest(self.reason)

    def decorate(self, function):
        if getattr(function, '_test_case', False):
            setattr(function, '__unittest_skip__', True)
            setattr(function, '__unittest_skip_why__', self.reason)
            return function
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
        self.message = str(message)
        self.args = (message,)

    def __str__(self):
        return self.message

class _Unexpected:
    def __init__(self, message):
        self.message = str(message)
        self.args = (message,)

    def __str__(self):
        return self.message

def expectedFailure(function):
    return _ExpectedFailure(function).call

class TestSuite:
    def __init__(self, tests=None):
        self.class_ = None
        self.tests = []
        if tests is not None:
            self.tests = list(tests)

    def addTest(self, test):
        self.tests = [*self.tests, test]

    def addTests(self, tests):
        for test in tests:
            self.addTest(test)

    def _fixture(self, name, result):
        if self.class_ is None or getattr(self.class_, '__unittest_skip__', False):
            return True
        fixture = getattr(self.class_, name, None)
        if fixture is None:
            return True
        outcome = __call_outcome(_Call(fixture, [self.class_], {}).invoke)
        if outcome[0]:
            return True
        entry = [name, _message(outcome[1], outcome[2]), __class_name(self.class_)]
        if isinstance(outcome[1], SkipTest):
            result.skipped = [*result.skipped, entry]
        else:
            result.errors = [*result.errors, entry]
        return False

    def run(self, result):
        if self._fixture('setUpClass', result):
            try:
                for test in self.tests:
                    test.run(result)
            finally:
                self._fixture('tearDownClass', result)
        return result

class TestLoader:
    def getTestCaseNames(self, cls):
        return _ordered([name for name in __class_methods(cls) if name[:4] == 'test'])

    def loadTestsFromTestCase(self, cls):
        methods = self.getTestCaseNames(cls)
        if len(methods) == 0 and getattr(cls, 'runTest', None) is not None:
            methods = ['runTest']
        suite = TestSuite([cls(name) for name in methods])
        suite.class_ = cls
        return suite

    def loadTestsFromModule(self, module, pattern=None):
        if module is None or module == '__main__':
            names = __program_namespace()
            module_name = '__main__'
        else:
            if type(module) == type(''):
                module = __load_module(module)
            names = __program_namespace(module)
            module_name = __class_name(module)
        suite = TestSuite()
        for name in _ordered(list(names)):
            cls = names[name]
            if isinstance(cls, TestCase) or not getattr(cls, '_test_case', False):
                continue
            tests = self.loadTestsFromTestCase(cls)
            for test in tests.tests:
                test._test_module = module_name
            suite.addTest(tests)
        return suite

    def discover(self, start_dir, pattern='test*.py', top_level_dir=None):
        raise 'NotImplementedError: test discovery needs filesystem access'


def main(module=None, exit=True, verbosity=1, argv=None, testRunner=None):
    return _main(module, exit, verbosity, argv, testRunner)


class _Call:
    def __init__(self, function, args, kwargs):
        self.function = function
        self.args = args
        self.kwargs = kwargs

    def invoke(self):
        return self.function(*self.args, **self.kwargs)


def _message(value, fallback=''):
    return getattr(value, 'message', str(value))


def _representation(value):
    return repr(value)


def _matches(pattern, text):
    # No expression module is carried by this source store. Seek the
    # phrase as written until that module is supplied.
    return pattern in text


class _SubTest:
    def __init__(self, case, message, parameters):
        self.case = case
        self.message = message
        self.parameters = parameters

    def __enter__(self):
        return None

    def __exit__(self, kind, value, traceback):
        if kind is None or self.case._result is None:
            return False
        detail = self.case._method
        if self.message is not None:
            detail += ' [' + str(self.message) + ']'
        words = ''
        for name in list(self.parameters):
            if words != '':
                words += ', '
            words += name + '=' + _representation(self.parameters[name])
        if words != '':
            detail += ' (' + words + ')'
        entry = [detail, _message(value), self.case.id()]
        result = self.case._result
        if isinstance(value, SkipTest):
            result.skipped = [*result.skipped, entry]
        elif isinstance(value, self.case.failureException):
            result.failures = [*result.failures, entry]
        else:
            result.errors = [*result.errors, entry]
        return True


class _Warns:
    def __init__(self, expected, pattern=None):
        self.expected = expected
        self.pattern = pattern
        self.warning = None
        self.filename = None
        self.lineno = None

    def __enter__(self):
        import warnings
        self.manager = warnings.catch_warnings(record=True, _internal=True)
        self.records = self.manager.__enter__()
        warnings.simplefilter('always', self.expected)
        return self

    def __exit__(self, kind, value, traceback):
        self.manager.__exit__(kind, value, traceback)
        if kind is not None:
            return False
        for record in self.manager.records:
            if isinstance(record.message, self.expected):
                if self.pattern is None or _matches(self.pattern, _message(record.message)):
                    self.warning = record.message
                    self.filename = record.filename
                    self.lineno = record.lineno
                    return False
        raise AssertionError(__class_name(self.expected) + ' not triggered')


def _lines(text):
    lines = []
    current = ''
    for index in range(len(text)):
        character = text[index]
        current += character
        if character == '\n':
            lines = [*lines, current]
            current = ''
    if current != '':
        lines = [*lines, current]
    return lines


def _line_diff(first, second):
    left = _lines(first)
    right = _lines(second)
    text = ''
    index = 0
    while index < len(left) or index < len(right):
        if index < len(left) and index < len(right) and left[index] == right[index]:
            text += '  ' + left[index]
        else:
            if index < len(left):
                text += '- ' + left[index]
            if index < len(right):
                text += '+ ' + right[index]
        index += 1
    return text


def _ordered(values):
    result = []
    for value in values:
        at = 0
        while at < len(result) and _word_before(result[at], value):
            at += 1
        result = [*result[:at], value, *result[at:]]
    return result


class TextTestRunner:
    def __init__(self, stream=None, descriptions=True, verbosity=1, failfast=False, buffer=False, resultclass=None, warnings=None, **kwargs):
        if buffer or warnings is not None or len(kwargs) != 0:
            raise 'NotImplementedError: these test runner options are not supported'
        self.stream = stream
        self.descriptions = descriptions
        self.verbosity = verbosity
        self.failfast = failfast
        self.resultclass = resultclass

    def _write(self, words):
        if self.stream is None:
            import sys
            print(words, end='', file=sys.stderr)
        else:
            self.stream.write(words)

    def _run(self, test, result):
        if isinstance(test, TestSuite):
            if test._fixture('setUpClass', result):
                try:
                    for member in test.tests:
                        self._run(member, result)
                        if self.failfast and not result.wasSuccessful():
                            break
                finally:
                    test._fixture('tearDownClass', result)
            return None
        failures = len(result.failures)
        errors = len(result.errors)
        skips = len(result.skipped)
        expected = len(result.expectedFailures)
        unexpected = len(result.unexpectedSuccesses)
        if self.verbosity > 1:
            self._write(test._method + ' (' + test.id() + ') ... ')
        test.run(result)
        mark = '.'
        description = 'ok'
        if len(result.errors) > errors:
            mark = 'E'
            description = 'ERROR'
        elif len(result.failures) > failures:
            mark = 'F'
            description = 'FAIL'
        elif len(result.skipped) > skips:
            mark = 's'
            description = 'skipped ' + repr(result.skipped[len(result.skipped) - 1][1])
        elif len(result.expectedFailures) > expected:
            mark = 'x'
            description = 'expected failure'
        elif len(result.unexpectedSuccesses) > unexpected:
            mark = 'u'
            description = 'unexpected success'
        if self.verbosity > 1:
            self._write(description + '\n')
        elif self.verbosity == 1:
            self._write(mark)

    def run(self, test):
        result = TestResult()
        if self.resultclass is not None:
            result = self.resultclass()
        started = __clock()
        self._run(test, result)
        elapsed = __clock() - started
        if self.verbosity > 0:
            self._write('\n')
        for error in result.errors:
            self._failure('ERROR', error, '')
        for failure in result.failures:
            self._failure('FAIL', failure, 'AssertionError: ')
        self._write('----------------------------------------------------------------------' + '\n')
        noun = ' tests'
        if result.testsRun == 1:
            noun = ' test'
        self._write('Ran ' + str(result.testsRun) + noun + ' in ' + ('%.3f' % elapsed) + 's\n\n')
        counts = []
        if len(result.failures) > 0:
            counts = [*counts, 'failures=' + str(len(result.failures))]
        if len(result.errors) > 0:
            counts = [*counts, 'errors=' + str(len(result.errors))]
        if len(result.skipped) > 0:
            counts = [*counts, 'skipped=' + str(len(result.skipped))]
        if len(result.expectedFailures) > 0:
            counts = [*counts, 'expected failures=' + str(len(result.expectedFailures))]
        if len(result.unexpectedSuccesses) > 0:
            counts = [*counts, 'unexpected successes=' + str(len(result.unexpectedSuccesses))]
        status = 'OK'
        if not result.wasSuccessful():
            status = 'FAILED'
        if len(counts) > 0:
            tail = ''
            for count in counts:
                if tail != '':
                    tail += ', '
                tail += count
            status += ' (' + tail + ')'
        self._write(status + '\n')
        return result

    def _failure(self, label, entry, prefix):
        self._write('======================================================================' + '\n')
        self._write(label + ': ' + entry[0] + ' (' + entry[2] + ')\n')
        self._write('----------------------------------------------------------------------' + '\n')
        self._write('Traceback (most recent call last):\n')
        self._write(prefix + entry[1] + '\n\n')


def enterModuleContext(context):
    raise 'NotImplementedError: module context cleanup is not supported'


def _main(module=None, exit=True, verbosity=1, argv=None, testRunner=None):
    if argv is not None and len(argv) > 1:
        raise 'NotImplementedError: selecting tests from command arguments is not supported'
    loader = TestLoader()
    suite = loader.loadTestsFromModule(module)
    if testRunner is None:
        testRunner = TextTestRunner(verbosity=verbosity)
    result = testRunner.run(suite)
    if exit:
        if not result.wasSuccessful():
            raise 'test run failed'
        __finish()
    return _TestProgram(result)


def _word_before(left, right):
    index = 0
    while index < len(left) and index < len(right):
        a = ord(left[index])
        b = ord(right[index])
        if a != b:
            return a < b
        index += 1
    return len(left) < len(right)


class _TestProgram:
    def __init__(self, result):
        self.result = result
