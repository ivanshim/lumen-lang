# Host primitives used by class bodies need non-private module bindings.
_host_call_outcome = __call_outcome
_host_class_methods = __class_methods
_host_clock = __clock
_host_load_module = __load_module
_host_program_namespace = __program_namespace
_host_clear_traceback = __trace_native__
_traceback_module = None

def _clear_exception_frames(traceback):
    if traceback is None:
        return
    global _traceback_module
    if _traceback_module is None:
        _traceback_module = _host_load_module('traceback', 'cached')
    if _traceback_module is not None:
        _traceback_module.clear_frames(traceback)
    elif not _host_clear_traceback(10, traceback):
        _traceback_module = _host_load_module('traceback')
        _traceback_module.clear_frames(traceback)

# Assertions and the test lifecycle live beside the modules they serve.
class SkipTest(Exception):
    pass


class _Context:
    def __enter__(self):
        return self

    def __exit__(self, kind, value, traceback):
        return False

class _Raises:
    def __init__(self, expected, phrase=None):
        self.expected = expected
        self.phrase = _compile_regex(phrase) if phrase is not None else None
        self.exception = None

    def __enter__(self):
        return self

    def __exit__(self, kind, value, traceback):
        if kind is None:
            raise AssertionError(_class_name(self.expected) + ' not raised')
        if not isinstance(value, self.expected):
            return False
        _clear_exception_frames(traceback)
        self.exception = value.with_traceback(None)
        # An exception's string is its own message, the way CPython's
        # assertRaisesRegex reads it; a `message` attribute on the
        # exception is not the same thing (argparse's ArgumentError
        # carries the raw text there and the argument name only in
        # __str__), so the pattern is matched against str(value).
        if self.phrase is not None and self.phrase.search(str(value)) is None:
            raise AssertionError('"' + self.phrase.pattern + '" does not match "' + str(value) + '"')
        return True

from unittest.result import TestResult

# One recorded outcome, a (test, message) pair as the reference keeps it;
# the report names it by its label and the place it came from.
class _Entry(tuple):
    def __new__(cls, test, message, label, where):
        entry = tuple.__new__(cls, (test, message))
        entry.label = label
        entry.where = where
        return entry

class TextTestResult(TestResult):
    def __repr__(self):
        return '<unittest.runner.TextTestResult run=%d errors=%d failures=%d>' % (self.testsRun, len(self.errors), len(self.failures))
class _NotWarns:
    def __init__(self, expected):
        self.expected = expected

    def __enter__(self):
        import warnings
        self.manager = warnings.catch_warnings(record=True)
        self.records = self.manager.__enter__()
        warnings.simplefilter('always', self.expected)
        return self

    def __exit__(self, kind, value, traceback):
        self.manager.__exit__(kind, value, traceback)
        if kind is not None:
            return False
        for record in self.records:
            if isinstance(record.message, self.expected):
                raise AssertionError(repr(record.message) + ' triggered')
        return False

class TestCase:
    _test_case = True
    failureException = AssertionError
    maxDiff = 640
    longMessage = True
    _class_cleanups = []

    def __init__(self, methodName='runTest'):
        self._method = methodName
        self._cleanups = []
        self._result = None
        self._type_equality_funcs = {}

    # Represent a case using its real class and selected method.
    def __repr__(self):
        cls = type(self)
        return '<%s.%s testMethod=%s>' % (cls.__module__, cls.__qualname__, self._method)

    @classmethod
    def setUpClass(cls):
        pass

    @classmethod
    def tearDownClass(cls):
        pass

    @classmethod
    def addClassCleanup(cls, function, /, *args, **kwargs):
        # Each concrete class owns its stack, including classes that inherit
        # fixture methods from another test case.
        if '_class_cleanups' not in cls.__dict__:
            cls._class_cleanups = []
        cls._class_cleanups.append((function, args, kwargs))

    @classmethod
    def enterClassContext(cls, cm):
        kind = type(cm)
        try:
            enter = kind.__enter__
            exit = kind.__exit__
        except AttributeError:
            raise TypeError(f"'{kind.__module__}.{kind.__qualname__}' object does not support the context manager protocol") from None
        result = enter(cm)
        cls.addClassCleanup(exit, cm, None, None, None)
        return result

    @classmethod
    def doClassCleanups(cls):
        import sys
        cls.tearDown_exceptions = []
        cleanups = cls.__dict__.get('_class_cleanups', [])
        while cleanups:
            function, args, kwargs = cleanups.pop()
            try:
                function(*args, **kwargs)
            except Exception:
                cls.tearDown_exceptions.append(sys.exc_info())

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

    # Register an equality assertion for exactly this operand type.
    def addTypeEqualityFunc(self, typeobj, function):
        self._type_equality_funcs[typeobj] = function

    def assertEqual(self, a, b, msg=None):
        if self._type_equality_funcs and type(a) is type(b):
            assertion = self._type_equality_funcs.get(type(a))
            if assertion is not None:
                if isinstance(assertion, str):
                    assertion = getattr(self, assertion)
                return assertion(a, b, msg=msg)
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
        return getattr(self, '_test_module', '__main__') + '.' + _class_name(self) + '.' + self._method

    def __str__(self):
        return self._method + ' (' + self.id() + ')'

    def assertNotEqual(self, a, b, msg=None):
        if not (a != b):
            self._check(False, str(a) + ' == ' + str(b), msg)

    def assertTrue(self, value, msg=None):
        if not value:
            self._check(False, str(value) + ' is not true', msg)

    def assertFalse(self, value, msg=None):
        if value:
            self._check(False, str(value) + ' is not false', msg)

    def assertIs(self, a, b, msg=None):
        if a is not b:
            self._check(False, str(a) + ' is not ' + str(b), msg)

    def assertIsNot(self, a, b, msg=None):
        if a is b:
            self._check(False, 'unexpected identity', msg)

    def assertIsNone(self, value, msg=None):
        self.assertIs(value, None, msg)

    def assertIsNotNone(self, value, msg=None):
        self.assertIsNot(value, None, msg)

    def assertIn(self, member, container, msg=None):
        if member not in container:
            self._check(False, str(member) + ' not found in ' + str(container), msg)

    def assertNotIn(self, member, container, msg=None):
        if member in container:
            self._check(False, str(member) + ' unexpectedly found', msg)

    def assertIsInstance(self, value, kind, msg=None):
        if not isinstance(value, kind):
            self._check(False, 'value is not an instance of the requested class', msg)

    def assertGreater(self, a, b, msg=None):
        if not (a > b):
            self._check(False, str(a) + ' not greater than ' + str(b), msg)

    def assertLess(self, a, b, msg=None):
        if not (a < b):
            self._check(False, str(a) + ' not less than ' + str(b), msg)

    def assertGreaterEqual(self, a, b, msg=None):
        if not (a >= b):
            self._check(False, str(a) + ' not greater than or equal to ' + str(b), msg)

    def assertLessEqual(self, a, b, msg=None):
        if not (a <= b):
            self._check(False, str(a) + ' not less than or equal to ' + str(b), msg)

    def assertAlmostEqual(self, a, b, places=None, msg=None, delta=None):
        if delta is not None and places is not None:
            raise 'TypeError: specify delta or places not both'
        if places is None:
            places = 7
        difference = abs(a - b)
        if delta is not None:
            close = difference <= delta
        else:
            close = difference < 0.5 * 10 ** (-places)
        if not (a == b or close):
            self._check(False, str(a) + ' != ' + str(b) + ' within tolerance', msg)

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
        try:
            args[0](*args[1:], **keywords)
        except BaseException as error:
            if not isinstance(error, expected):
                raise
            _clear_exception_frames(error.__traceback__)
            error.with_traceback(None)
            return
        raise AssertionError(_class_name(expected) + ' not raised')

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

    def enterContext(self, cm):
        # The context is entered now and left when the test is cleaned
        # up, so a test may use one without a with statement around it.
        result = cm.__enter__()
        self.addCleanup(cm.__exit__, None, None, None)
        return result

    def doCleanups(self):
        successful = True
        while len(self._cleanups) > 0:
            cleanup = self._cleanups[len(self._cleanups) - 1]
            self._cleanups = self._cleanups[:-1]
            outcome = _host_call_outcome(_Call(cleanup[0], cleanup[1], cleanup[2]).invoke)
            if not outcome[0]:
                successful = False
                if self._result is None:
                    raise outcome[1]
                self._result.errors = [*self._result.errors, _Entry(self, _message(outcome[1], outcome[2]), self._method, self.id())]
        return successful

    def countTestCases(self):
        return 1

    def __call__(self, result=None):
        return self.run(result)

    def shortDescription(self):
        return None

    def assertIsSubclass(self, cls, superclass, msg=None):
        if not issubclass(cls, superclass):
            self._check(False, _representation(cls) + ' is not a subclass of ' + _representation(superclass), msg)

    def assertNotIsSubclass(self, cls, superclass, msg=None):
        if issubclass(cls, superclass):
            self._check(False, _representation(cls) + ' is a subclass of ' + _representation(superclass), msg)

    def assertNotIsInstance(self, value, kind, msg=None):
        if isinstance(value, kind):
            self._check(False, _representation(value) + ' is an instance of the requested class', msg)

    def assertNotAlmostEqual(self, a, b, places=None, msg=None, delta=None):
        if delta is not None and places is not None:
            raise 'TypeError: specify delta or places not both'
        if places is None:
            places = 7
        difference = abs(a - b)
        if delta is not None:
            close = difference <= delta
        else:
            close = difference < 0.5 * 10 ** (-places)
        if a == b or close:
            self._check(False, _representation(a) + ' == ' + _representation(b) + ' within tolerance', msg)

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
        pattern = _compile_regex(pattern)
        if pattern.search(text) is None:
            message = "Regex didn't match: " + _representation(pattern.pattern) + ' not found in ' + _representation(text)
            self._check(False, message, msg)

    def assertNotRegex(self, text, pattern, msg=None):
        pattern = _compile_regex(pattern)
        match = pattern.search(text)
        if match is not None:
            message = 'Regex matched: ' + _representation(text[match.start():match.end()]) + ' matches ' + _representation(pattern.pattern) + ' in ' + _representation(text)
            self._check(False, message, msg)

    def assertWarns(self, expected, *args, **kwargs):
        context = _Warns(expected)
        if len(args) == 0:
            return context
        with context:
            args[0](*args[1:], **kwargs)

    def _assertNotWarns(self, expected, *args, **kwargs):
        context = _NotWarns(expected)
        if len(args) == 0:
            return context
        with context:
            args[0](*args[1:], **kwargs)

    def assertWarnsRegex(self, expected_warning, expected_regex, *args, **kwargs):
        context = _Warns(expected_warning, expected_regex)
        if len(args) == 0:
            return context
        with context:
            args[0](*args[1:], **kwargs)

    def assertLogs(self, logger=None, level=None):
        raise 'NotImplementedError: log capture is not supported'

    def assertHasAttr(self, obj, name, msg=None):
        marker = _Context()
        if getattr(obj, name, marker) is marker:
            self._check(False, 'attribute ' + _representation(name) + ' is missing', msg)

    def assertNotHasAttr(self, obj, name, msg=None):
        marker = _Context()
        if getattr(obj, name, marker) is not marker:
            self._check(False, 'attribute ' + _representation(name) + ' is present', msg)

    def assertStartsWith(self, text, prefix, msg=None):
        if not (text[:len(prefix)] == prefix):
            self._check(False, _representation(text) + ' does not start with ' + _representation(prefix), msg)

    def assertNotStartsWith(self, text, prefix, msg=None):
        if not (text[:len(prefix)] != prefix):
            self._check(False, _representation(text) + ' starts with ' + _representation(prefix), msg)

    def assertEndsWith(self, text, suffix, msg=None):
        if not (suffix == '' or text[-len(suffix):] == suffix):
            self._check(False, _representation(text) + ' does not end with ' + _representation(suffix), msg)

    def _tail_type_check(self, text, tails, msg):
        # A prefix or suffix handed a kind it cannot compare fails with
        # the kind named, the way CPython's own tail check does.
        if not isinstance(tails, tuple):
            tails = (tails,)
        for tail in tails:
            if isinstance(tail, str):
                if not isinstance(text, str):
                    self._check(False, 'Expected str, not ' + _class_name(text), msg)
            elif isinstance(tail, (bytes, bytearray)):
                if not isinstance(text, (bytes, bytearray)):
                    self._check(False, 'Expected bytes, not ' + _class_name(text), msg)

    def assertNotEndsWith(self, text, suffix, msg=None):
        try:
            if not text.endswith(suffix):
                return
        except (AttributeError, TypeError):
            self._tail_type_check(text, suffix, msg)
            raise
        if isinstance(suffix, tuple):
            for part in suffix:
                if text.endswith(part):
                    suffix = part
                    break
        self._check(False, _representation(text) + ' ends with ' + _representation(suffix), msg)

    def _run_test(self):
        if getattr(self, '__unittest_skip__', False):
            self.skipTest(getattr(self, '__unittest_skip_why__', 'skipped'))
        self.setUp()
        try:
            outcome = _host_call_outcome(getattr(self, self._method))
            if not outcome[0]:
                if isinstance(outcome[1], BaseException):
                    raise outcome[1]
                raise RuntimeError(outcome[2])
        finally:
            self.tearDown()

    def _callTestMethod(self, method):
        returned = method()
        if returned is not None:
            import warnings
            warnings.warn('It is deprecated to return a value that is not None from a test case',
                          DeprecationWarning, stacklevel=2)

    def run(self, result=None):
        if result is None:
            result = TestResult()
        self._result = result
        result.testsRun += 1
        outcome = _host_call_outcome(self._run_test)
        self.doCleanups()
        if outcome[0]:
            return result
        error = outcome[1]
        entry = _Entry(self, getattr(error, 'message', outcome[2]), self._method, self.id())
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

class IsolatedAsyncioTestCase(TestCase):
    # The host can drive a coroutine which completes without suspension.
    # Suspended I/O requires an event loop and is refused explicitly.
    def asyncSetUp(self):
        pass

    def asyncTearDown(self):
        pass

    def _callMaybeAsync(self, function, *args, **kwargs):
        import inspect
        if not inspect.iscoroutinefunction(function):
            return function(*args, **kwargs)
        pending = function(*args, **kwargs)
        try:
            try:
                pending.send(None)
            except StopIteration as stopped:
                return stopped.value
            raise NotImplementedError('asynchronous suspension requires an event loop')
        finally:
            pending.close()

    def _callTestMethod(self, method):
        returned = self._callMaybeAsync(method)
        if returned is not None:
            import warnings
            warnings.warn('It is deprecated to return a value that is not None from a test case',
                          DeprecationWarning, stacklevel=2)

    def _run_test(self):
        if getattr(self, '__unittest_skip__', False):
            self.skipTest(getattr(self, '__unittest_skip_why__', 'skipped'))
        self.setUp()
        self._callMaybeAsync(self.asyncSetUp)
        try:
            self._callTestMethod(getattr(self, self._method))
        finally:
            self._callMaybeAsync(self.asyncTearDown)
            self.tearDown()


class _Skip:
    def __init__(self, reason):
        self.reason = reason

    def call(self, *args, **kwargs):
        raise SkipTest(self.reason)

    def decorate(self, function):
        if isinstance(function, type) or getattr(function, '_test_case', False):
            setattr(function, '__unittest_skip__', True)
            setattr(function, '__unittest_skip_why__', self.reason)
            return function
        def skipped(*args, _reason=self.reason, **kwargs):
            raise SkipTest(_reason)
        setattr(skipped, '__unittest_skip__', True)
        setattr(skipped, '__unittest_skip_why__', self.reason)
        return skipped

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

class _Expected(Exception):
    pass


class _Unexpected(Exception):
    pass


def expectedFailure(function):
    def expected(*args, _function=function, **kwargs):
        try:
            _function(*args, **kwargs)
        except SkipTest:
            raise
        except:
            raise _Expected('expected failure')
        raise _Unexpected('unexpected success')
    return expected

class TestSuite:
    _removed_tests = 0
    def __init__(self, tests=None):
        self.class_ = None
        self.module_namespace = None
        self.tests = []
        if tests is not None:
            self.tests = list(tests)

    def addTest(self, test):
        self.tests = [*self.tests, test]

    def __iter__(self):
        return iter(self.tests)

    def addTests(self, tests):
        if isinstance(tests, TestSuite):
            self.tests = [*self.tests, tests]
            return
        for test in tests:
            self.addTest(test)

    def countTestCases(self):
        return self._removed_tests + sum(test.countTestCases() for test in self.tests if test is not None)

    def __call__(self, result):
        return self.run(result)
    def _module_fixture(self, name, result):
        if self.module_namespace is None:
            return True
        fixture = doModuleCleanups if name == 'doModuleCleanups' else self.module_namespace.get(name)
        if fixture is None:
            return True
        outcome = _host_call_outcome(fixture)
        if outcome[0]:
            return True
        entry = _Entry(name, _message(outcome[1], outcome[2]), name, self.module_namespace.get('__name__', ''))
        if isinstance(outcome[1], SkipTest):
            result.skipped = [*result.skipped, entry]
        else:
            result.errors = [*result.errors, entry]
        return False

    def _fixture(self, name, result):
        if self.class_ is None or getattr(self.class_, '__unittest_skip__', False):
            return True
        fixture = getattr(self.class_, name, None)
        if fixture is None:
            return True
        outcome = _host_call_outcome(fixture)
        if outcome[0]:
            if name == 'doClassCleanups':
                faults = getattr(self.class_, 'tearDown_exceptions', [])
                for kind, value, traceback in faults:
                    result.errors = [*result.errors, _Entry(name, _message(value), name, _class_name(self.class_))]
                return len(faults) == 0
            return True
        entry = _Entry(name, _message(outcome[1], outcome[2]), name, _class_name(self.class_))
        if isinstance(outcome[1], SkipTest):
            result.skipped = [*result.skipped, entry]
        else:
            result.errors = [*result.errors, entry]
        return False

    def run(self, result):
        TextTestRunner(verbosity=0)._run(self, result)
        return result

class _FailedTest(TestCase):
    def __init__(self, name, error):
        super().__init__('runTest')
        self._test_module = name
        self._error = error

    def runTest(self):
        raise self._error


class TestLoader:
    suiteClass = TestSuite
    testMethodPrefix = 'test'
    testNamePatterns = None
    sortTestMethodsUsing = staticmethod(lambda a, b: (a > b) - (a < b))
    _top_level_dir = None

    def __init__(self):
        self.errors = []
        self._loading_packages = set()

    def getTestCaseNames(self, cls):
        # dir includes every base in the C3 order; getattr resolves overrides,
        # including non-callable attributes that hide inherited test methods.
        names = []
        for name in dir(cls):
            if not name.startswith(self.testMethodPrefix) or not callable(getattr(cls, name)):
                continue
            if self.testNamePatterns is not None:
                from fnmatch import fnmatchcase
                full = cls.__module__ + '.' + cls.__qualname__ + '.' + name
                if not any(fnmatchcase(full, pattern) for pattern in self.testNamePatterns):
                    continue
            names.append(name)
        if self.sortTestMethodsUsing:
            compare = self.sortTestMethodsUsing
            for position in range(1, len(names)):
                name = names[position]
                cursor = position
                while cursor and compare(name, names[cursor - 1]) < 0:
                    names[cursor] = names[cursor - 1]
                    cursor -= 1
                names[cursor] = name
        return names

    def _failed(self, name, error):
        self.errors.append(str(error))
        return self.suiteClass([_FailedTest(name, error)])

    def loadTestsFromTestCase(self, cls):
        if issubclass(cls, TestSuite):
            raise TypeError('Test cases should not be derived from TestSuite')
        if cls is TestCase or getattr(cls, '__abstractmethods__', None):
            return self.suiteClass()
        methods = self.getTestCaseNames(cls)
        if len(methods) == 0 and hasattr(cls, 'runTest'):
            methods = ['runTest']
        suite = self.suiteClass([cls(name) for name in methods])
        suite.class_ = cls
        return suite

    def loadTestsFromModule(self, module, pattern=None):
        if module is None or module == '__main__':
            names = _host_program_namespace()
            module_name = '__main__'
        else:
            if type(module) == type(''):
                module = _host_load_module(module)
            names = {name: getattr(module, name) for name in dir(module)}
            module_name = _class_name(module)
        suite = self.suiteClass()
        suite.module_namespace = names
        for name in _ordered(list(names)):
            # The host's main frame also contains private compiler bindings.
            if module_name == '__main__' and (name.startswith('#') or name.startswith('\0')):
                continue
            cls = names[name]
            if not isinstance(cls, type) or not issubclass(cls, TestCase) or cls is TestCase:
                continue
            tests = self.loadTestsFromTestCase(cls)
            for test in tests:
                test._test_module = module_name
            suite.addTest(tests)
        hook = names.get('load_tests')
        if hook is not None:
            try:
                return hook(self, suite, pattern)
            except Exception as error:
                return self._failed(module_name, error)
        return suite

    def loadTestsFromNames(self, names, module=None):
        return self.suiteClass(self.loadTestsFromName(name, module) for name in names)

    def loadTestsFromName(self, name, module=None):
        parts = name.split('.')
        if module is None:
            for length in range(len(parts), 0, -1):
                try:
                    module = _host_load_module('.'.join(parts[:length]))
                    parts = parts[length:]
                    break
                except ImportError as error:
                    if length == 1:
                        return self._failed(name, error)
        obj = module
        parent = None
        for part in parts:
            parent = obj
            try:
                obj = getattr(obj, part)
            except AttributeError as error:
                return self._failed(name, error)
        if isinstance(obj, type) and issubclass(obj, TestCase):
            return self.loadTestsFromTestCase(obj)
        if isinstance(parent, type) and issubclass(parent, TestCase):
            return self.suiteClass([parent(parts[-1])])
        if isinstance(obj, TestSuite):
            return obj
        if callable(obj):
            result = obj()
            if isinstance(result, TestSuite):
                return result
            if isinstance(result, TestCase):
                return self.suiteClass([result])
            raise TypeError('calling %s returned %s, not a test' % (obj, result))
        import types
        if isinstance(obj, types.ModuleType):
            return self.loadTestsFromModule(obj)
        raise TypeError("don't know how to make test from: %s" % obj)

    def discover(self, start_dir, pattern='test*.py', top_level_dir=None):
        import os
        import sys
        previous = self._top_level_dir
        implicit = top_level_dir is None and previous is None
        top = top_level_dir if top_level_dir is not None else previous
        if not os.path.isdir(start_dir):
            module = _host_load_module(start_dir)
            start_dir = os.path.dirname(module.__file__)
            if implicit:
                top = start_dir
                for part in module.__name__.split('.'):
                    top = os.path.dirname(top)
        start_dir = os.path.abspath(start_dir)
        top = os.path.abspath(top if top is not None else start_dir)
        if top not in sys.path:
            sys.path.insert(0, top)
        self._top_level_dir = top
        try:
            if start_dir != top and not os.path.isfile(os.path.join(start_dir, '__init__.py')):
                raise ImportError('Start directory is not importable: %r' % start_dir)
            return self.suiteClass(self._find_tests(start_dir, pattern))
        finally:
            self._top_level_dir = previous

    def _get_name_from_path(self, path):
        import os
        path = os.path.abspath(path)
        if path == self._top_level_dir:
            return '.'
        prefix = self._top_level_dir.rstrip('/') + '/'
        if not path.startswith(prefix):
            raise AssertionError('Path must be within the project')
        return os.path.splitext(path[len(prefix):])[0].replace('/', '.')

    def _match_path(self, path, full_path, pattern):
        from fnmatch import fnmatch
        return fnmatch(path, pattern)

    def _find_tests(self, start_dir, pattern):
        import os
        name = self._get_name_from_path(start_dir)
        if name != '.' and name not in self._loading_packages:
            tests, recurse = self._find_test_path(start_dir, pattern)
            if tests is not None:
                yield tests
            if not recurse:
                return
        for path in sorted(os.listdir(start_dir)):
            full = os.path.join(start_dir, path)
            tests, recurse = self._find_test_path(full, pattern)
            if tests is not None:
                yield tests
            if recurse:
                name = self._get_name_from_path(full)
                self._loading_packages.add(name)
                try:
                    yield from self._find_tests(full, pattern)
                finally:
                    self._loading_packages.discard(name)

    def _find_test_path(self, full_path, pattern):
        import os
        package = os.path.isdir(full_path)
        basename = os.path.basename(full_path)
        if package:
            if not os.path.isfile(os.path.join(full_path, '__init__.py')):
                return None, False
        elif not basename.endswith('.py') or not _spellable(basename[:-3]) or not self._match_path(basename, full_path, pattern):
            return None, False
        name = self._get_name_from_path(full_path)
        try:
            module = _host_load_module(name)
        except SkipTest as error:
            return self.suiteClass([_FailedTest(name, error)]), False
        except Exception as error:
            return self._failed(name, error), False
        if package:
            self._loading_packages.add(name)
        try:
            tests = self.loadTestsFromModule(module, pattern=pattern)
            return tests, package and getattr(module, 'load_tests', None) is None
        finally:
            if package:
                self._loading_packages.discard(name)


TestLoader.loadTestsFromModule._honours_load_tests = True
defaultTestLoader = TestLoader()

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


def _compile_regex(pattern):
    # A phrase handed to an assertRaisesRegex/assertRegex family method is
    # either an already-compiled pattern (it answers to `search`) or plain
    # text, which is compiled through the real `re` module the way
    # CPython's own unittest compiles it.
    if getattr(pattern, 'search', None) is not None:
        return pattern
    if type(pattern) is str:
        literal = _literal_text(pattern)
        if literal is not None:
            return _LiteralPattern(pattern, literal)
    import re
    return re.compile(pattern)


# What a pattern spells when it holds nothing but plain characters and
# backslash-escaped punctuation (so the whole regular expression engine,
# whose compiling dwarfs the matching, has nothing to add); None otherwise.
def _literal_text(pattern):
    parts = []
    for piece in pattern.split('\\\\'):
        pieces = piece.split('\\')
        for at in range(len(pieces)):
            part = pieces[at]
            if at:
                if not part or part[0].isalnum() or not part[0].isascii():
                    return None
                part = part[1:]
            for meta in '.^$*+?{}[]|()':
                if meta in part:
                    return None
        parts.append(''.join(pieces))
    return '\\'.join(parts)


class _LiteralMatch:
    def __init__(self, string, begin, end):
        self.string = string
        self._span = (begin, end)

    def start(self, group=0):
        return self._span[0]

    def end(self, group=0):
        return self._span[1]

    def span(self, group=0):
        return self._span

    def group(self, group=0):
        return self.string[self._span[0]:self._span[1]]

    def __bool__(self):
        return True


class _LiteralPattern:
    def __init__(self, pattern, literal):
        self.pattern = pattern
        self._literal = literal

    def search(self, string):
        if not isinstance(string, str):
            import re
            return re.compile(self.pattern).search(string)
        begin = string.find(self._literal)
        if begin < 0:
            return None
        return _LiteralMatch(string, begin, begin + len(self._literal))


def _matches(pattern, text):
    return _compile_regex(pattern).search(text) is not None


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
        entry = _Entry(detail, _message(value), detail, self.case.id())
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
        self.pattern = _compile_regex(pattern) if pattern is not None else None
        self.warning = None
        self.filename = None
        self.lineno = None

    def __enter__(self):
        import warnings
        self.manager = warnings.catch_warnings(record=True)
        self.records = self.manager.__enter__()
        self.warnings = self.records
        warnings.simplefilter('always', self.expected)
        return self

    def __exit__(self, kind, value, traceback):
        self.manager.__exit__(kind, value, traceback)
        if kind is not None:
            return False
        import warnings
        first_matching = None
        matched = False
        non_matching = []
        for record in self.records:
            if isinstance(record.message, self.expected):
                if first_matching is None:
                    first_matching = record.message
                if self.pattern is not None and self.pattern.search(_message(record.message)) is None:
                    non_matching = [*non_matching, record]
                    continue
                if not matched:
                    matched = True
                    self.warning = record.message
                    self.filename = record.filename
                    self.lineno = record.lineno
            else:
                non_matching = [*non_matching, record]
        # Preserve other warning categories for an enclosing recorder.
        for record in non_matching:
            warnings.warn_explicit(record.message, record.category, record.filename, record.lineno)
        if matched:
            return False
        if first_matching is not None:
            raise AssertionError('"' + self.pattern.pattern + '" does not match "' + _message(first_matching) + '"')
        raise AssertionError(_class_name(self.expected) + ' not triggered')


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


# Order module names through the builtin Unicode string comparison.
def _ordered(values):
    return sorted(values)


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
            if not test._module_fixture('setUpModule', result):
                test._module_fixture('doModuleCleanups', result)
                return None
            # A base class with no test methods is still collected as an
            # empty child suite. Class fixtures belong to suites that hold
            # cases, matching the runner that skips setup when nothing runs.
            try:
                if test.countTestCases() > 0 and test._fixture('setUpClass', result):
                    try:
                        members = test.tests
                        for at in range(len(members)):
                            member = members[at]
                            self._run(member, result)
                            if self.failfast and not result.wasSuccessful():
                                break
                            # A test that has run is let go, as CPython's suite does,
                            # so what it holds is not kept alive to the end of the run.
                            if hasattr(member, 'countTestCases'):
                                test._removed_tests += member.countTestCases()
                            members[at] = None
                            member = None
                    finally:
                        test._fixture('tearDownClass', result)
            finally:
                try:
                    test._fixture('doClassCleanups', result)
                finally:
                    try:
                        test._module_fixture('tearDownModule', result)
                    finally:
                        test._module_fixture('doModuleCleanups', result)
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
            mark = 'F'  # An unexpected success is a failing test result.
            description = 'unexpected success'
        if self.verbosity > 1:
            self._write(description + '\n')
        elif self.verbosity == 1:
            # Reports written during a test must not split the result marks.
            self._progress += mark

    def run(self, test):
        self._progress = ""
        result = TextTestResult()
        if self.resultclass is not None:
            result = self.resultclass()
        started = _host_clock()
        self._run(test, result)
        elapsed = _host_clock() - started
        if self.verbosity == 1:
            self._write(self._progress)
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
        test = entry[0]
        name = getattr(entry, 'label', getattr(test, '_method', str(test)))
        where = getattr(entry, 'where', test.id() if hasattr(test, 'id') else '')
        self._write(label + ': ' + name + ' (' + where + ')\n')
        self._write('----------------------------------------------------------------------' + '\n')
        self._write('Traceback (most recent call last):\n')
        self._write(prefix + entry[1] + '\n\n')


# Whether a name is one a program could have written, as against one
# the host keeps for its own workings.
def _spellable(name):
    first = name[:1]
    if not (first == '_' or first.isalpha()):
        return False
    for letter in list(name):
        if not (letter == '_' or letter.isalnum()):
            return False
    return True


# The name of a class, or of the class of a thing.
def _class_name(value):
    name = getattr(value, '__name__', None)
    if name is None:
        name = type(value).__name__
    return name


_module_cleanups = []

def addModuleCleanup(function, /, *args, **kwargs):
    _module_cleanups.append((function, args, kwargs))

def enterModuleContext(cm):
    cls = type(cm)
    try:
        enter = cls.__enter__
        exit = cls.__exit__
    except AttributeError:
        raise TypeError("'" + cls.__module__ + '.' + cls.__qualname__ + "' object does not support the context manager protocol") from None
    result = enter(cm)
    addModuleCleanup(exit, cm, None, None, None)
    return result

def doModuleCleanups():
    errors = []
    while _module_cleanups:
        function, args, kwargs = _module_cleanups.pop()
        try:
            function(*args, **kwargs)
        except Exception as error:
            errors.append(error)
    if errors:
        raise errors[0]


def _reference_doctest_module(module):
    if module is not None and module != '__main__':
        return module
    names = _host_program_namespace()
    if '__test__' not in names or 'load_tests' not in names:
        return module
    import sys
    directory, _, filename = names.get('__file__', '').rpartition('/')
    version = '.'.join(str(part) for part in sys.version_info[:3])
    if ('/tests/' not in directory or not directory.endswith('/python-' + version)
            or not filename.startswith('test_') or not filename.endswith('.py')):
        return module
    # CPython's regrtest imports reference doctests under test.test_x.
    # The direct-file checker must collect that same module identity.
    import test
    if directory not in test.__path__:
        test.__path__.insert(0, directory)
    return _host_load_module('test.' + filename[:-3])


def _main(module=None, exit=True, verbosity=1, argv=None, testRunner=None):
    if argv is not None and len(argv) > 1:
        raise 'NotImplementedError: selecting tests from command arguments is not supported'
    loader = TestLoader()
    suite = loader.loadTestsFromModule(_reference_doctest_module(module))
    only = _only_classes()
    if only is not None:
        namespace = suite.module_namespace
        selected = _select_tests(suite, only)
        suite = TestSuite() if selected is None else selected
        suite.module_namespace = namespace
        # Named CI selections always emit one progress marker per method,
        # even when a reference file explicitly requests verbose output.
        verbosity = 1
    if testRunner is None:
        testRunner = TextTestRunner(verbosity=verbosity)
    result = testRunner.run(suite)
    if exit:
        if not result.wasSuccessful():
            raise SystemExit('test run failed')
        __finish()
    return _TestProgram(result)


def _select_tests(suite, names):
    if not isinstance(suite, TestSuite):
        cls = type(suite).__name__
        method = getattr(suite, '_method', '')
        return suite if cls in names or cls + '.' + method in names or suite.id() in names else None
    selected = TestSuite()
    selected.class_ = suite.class_
    # Each module suite carries its own fixtures. A selection must keep
    # that namespace so setUpModule still runs for the chosen classes.
    selected.module_namespace = suite.module_namespace
    for test in suite:
        kept = _select_tests(test, names)
        if kept is not None:
            selected.addTest(kept)
    return selected if selected.countTestCases() else None


def _only_classes():
    import os
    names = os.environ.get('LUMEN_UNITTEST_ONLY', '')
    if names == '':
        return None
    return names.split(',')




class _TestProgram:
    def __init__(self, result):
        self.result = result
