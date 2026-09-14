"""Interactive examples found in docstrings, run and checked.

A docstring may show what the interpreter prints:

    >>> 1 + 1
    2

This module finds such examples, runs each one, and compares what it
prints with the text written under it. The examples of one docstring
become one unittest test, so a test file can hand them to a suite:

    def load_tests(loader, tests, pattern):
        tests.addTest(doctest.DocTestSuite())
        return tests
"""

import sys
import unittest


# Option flags. The numbers are CPython's, so a file that adds two of them
# together, or prints one, sees the number it expects.
DONT_ACCEPT_TRUE_FOR_1 = 1
DONT_ACCEPT_BLANKLINE = 2
NORMALIZE_WHITESPACE = 4
ELLIPSIS = 8
SKIP = 16
IGNORE_EXCEPTION_DETAIL = 32

COMPARISON_FLAGS = (DONT_ACCEPT_TRUE_FOR_1 | DONT_ACCEPT_BLANKLINE |
                    NORMALIZE_WHITESPACE | ELLIPSIS | SKIP |
                    IGNORE_EXCEPTION_DETAIL)

REPORT_UDIFF = 64
REPORT_CDIFF = 128
REPORT_NDIFF = 256
REPORT_ONLY_FIRST_FAILURE = 512
FAIL_FAST = 1024

REPORTING_FLAGS = (REPORT_UDIFF | REPORT_CDIFF | REPORT_NDIFF |
                   REPORT_ONLY_FIRST_FAILURE | FAIL_FAST)

OPTIONFLAGS_BY_NAME = {
    'DONT_ACCEPT_TRUE_FOR_1': DONT_ACCEPT_TRUE_FOR_1,
    'DONT_ACCEPT_BLANKLINE': DONT_ACCEPT_BLANKLINE,
    'NORMALIZE_WHITESPACE': NORMALIZE_WHITESPACE,
    'ELLIPSIS': ELLIPSIS,
    'SKIP': SKIP,
    'IGNORE_EXCEPTION_DETAIL': IGNORE_EXCEPTION_DETAIL,
    'REPORT_UDIFF': REPORT_UDIFF,
    'REPORT_CDIFF': REPORT_CDIFF,
    'REPORT_NDIFF': REPORT_NDIFF,
    'REPORT_ONLY_FIRST_FAILURE': REPORT_ONLY_FIRST_FAILURE,
    'FAIL_FAST': FAIL_FAST,
}

ELLIPSIS_MARKER = '...'
BLANKLINE_MARKER = '<BLANKLINE>'

PS1 = '>>>'
PS2 = '...'

_TRACEBACK_HEADERS = ['Traceback (most recent call last):',
                      'Traceback (innermost last):']

# A line opening one of these begins a statement, never an expression, so
# its value is never shown.
_STATEMENT_WORDS = ['assert', 'async', 'await', 'break', 'class', 'continue',
                    'def', 'del', 'elif', 'else', 'except', 'finally', 'for',
                    'from', 'global', 'if', 'import', 'nonlocal', 'pass',
                    'raise', 'return', 'try', 'while', 'with', 'yield']

_OPEN_BRACKETS = ['(', '[', '{']
_CLOSE_BRACKETS = [')', ']', '}']

_unittest_reportflags = 0


def register_optionflag(name):
    flag = OPTIONFLAGS_BY_NAME.get(name)
    if flag is None:
        flag = 1
        for known in OPTIONFLAGS_BY_NAME:
            value = OPTIONFLAGS_BY_NAME[known]
            if value >= flag:
                flag = value * 2
        OPTIONFLAGS_BY_NAME[name] = flag
    return flag


def set_unittest_reportflags(flags):
    global _unittest_reportflags
    old = _unittest_reportflags
    _unittest_reportflags = flags
    return old


class DocTestFailure(Exception):
    def __init__(self, test, example, got):
        self.test = test
        self.example = example
        self.got = got


class UnexpectedException(Exception):
    def __init__(self, test, example, exc_info):
        self.test = test
        self.example = example
        self.exc_info = exc_info


class TestResults:
    """What testmod and testfile report: how many examples were run and
    how many of them failed."""

    def __init__(self, failed, attempted):
        self.failed = failed
        self.attempted = attempted

    def __len__(self):
        return 2

    def __getitem__(self, index):
        if index == 0:
            return self.failed
        if index == 1:
            return self.attempted
        raise IndexError('index out of range')

    def __repr__(self):
        return ('TestResults(failed=' + str(self.failed) +
                ', attempted=' + str(self.attempted) + ')')


class Example:
    """One `>>>` line, its continuation, and the text expected under it."""

    def __init__(self, source, want, exc_msg=None, lineno=0, indent=0,
                 options=None):
        if source != '' and source[-1] != '\n':
            source = source + '\n'
        if want != '' and want[-1] != '\n':
            want = want + '\n'
        self.source = source
        self.want = want
        self.exc_msg = exc_msg
        self.lineno = lineno
        self.indent = indent
        if options is None:
            options = {}
        self.options = options


class DocTest:
    """The examples of one docstring, with the namespace they run in."""

    def __init__(self, examples, globs, name, filename=None, lineno=None,
                 docstring=None):
        self.examples = examples
        self.globs = _copy_dict(globs)
        self.name = name
        self.filename = filename
        self.lineno = lineno
        self.docstring = docstring

    def __repr__(self):
        return '<DocTest ' + self.name + ' with ' + str(len(self.examples)) + ' examples>'


def _copy_dict(source):
    copy = {}
    if source is None:
        return copy
    for key in source:
        copy[key] = source[key]
    return copy


# ---------------------------------------------------------------- parsing

def _is_prompt(text, prompt):
    if text[:len(prompt)] != prompt:
        return False
    rest = text[len(prompt):]
    return rest == '' or rest[0] == ' '


def _after_prompt(text):
    # The prompt is three characters and one space; a bare prompt has no space.
    if len(text) <= 4:
        return ''
    return text[4:]


def _find_from(text, needle, start, stop):
    limit = stop - len(needle)
    index = start
    while index <= limit:
        if text[index:index + len(needle)] == needle:
            return index
        index += 1
    return -1


def _leading_spaces(line):
    count = 0
    while count < len(line) and line[count] == ' ':
        count += 1
    return count


def _directive_options(source):
    """The `# doctest: +FLAG, -FLAG` comment written on an example."""
    options = {}
    for line in source.split('\n'):
        at_hash = -1
        index = 0
        while index < len(line):
            if line[index] == '#':
                at_hash = index
                break
            index += 1
        if at_hash < 0:
            continue
        comment = line[at_hash + 1:]
        at_word = _find_from(comment, 'doctest:', 0, len(comment))
        if at_word < 0:
            continue
        for word in comment[at_word + 8:].split(','):
            word = word.strip()
            if len(word) < 2:
                continue
            sign = word[0]
            if sign != '+' and sign != '-':
                continue
            flag = OPTIONFLAGS_BY_NAME.get(word[1:])
            if flag is None:
                continue        # an unknown directive is left alone
            options[flag] = sign == '+'
    return options


def _starts_want_break(line, indent):
    """Whether a line ends the expected output of an example."""
    if line.strip() == '':
        return True
    if _leading_spaces(line) < indent:
        return True
    return _is_prompt(line[indent:], PS1)


class DocTestParser:
    """Turns the text of a docstring into Examples."""

    def parse(self, string, name='<string>'):
        return self.get_examples(string, name)

    def get_examples(self, string, name='<string>'):
        examples = []
        lines = string.split('\n')
        total = len(lines)
        index = 0
        while index < total:
            line = lines[index]
            stripped = line.lstrip()
            if not _is_prompt(stripped, PS1):
                index += 1
                continue
            indent = len(line) - len(stripped)
            lineno = index + 1
            source_lines = [_after_prompt(stripped)]
            index += 1
            while index < total:
                rest = lines[index][indent:]
                if not _is_prompt(rest, PS2):
                    break
                source_lines.append(_after_prompt(rest))
                index += 1
            want_lines = []
            while index < total:
                if _starts_want_break(lines[index], indent):
                    break
                want_lines.append(lines[index][indent:])
                index += 1
            source = '\n'.join(source_lines)
            want = ''
            if len(want_lines) > 0:
                want = '\n'.join(want_lines) + '\n'
            examples.append(Example(source, want, None, lineno, indent,
                                    _directive_options(source)))
        return examples

    def get_doctest(self, string, globs, name, filename, lineno):
        return DocTest(self.get_examples(string, name), globs, name,
                       filename, lineno, string)


# ---------------------------------------------------------------- finding

class DocTestFinder:
    """Collects the docstrings of a module that hold examples."""

    def __init__(self, verbose=False, parser=None, recurse=True,
                 exclude_empty=True):
        self.verbose = verbose
        if parser is None:
            parser = DocTestParser()
        self._parser = parser
        self._recurse = recurse
        self._exclude_empty = exclude_empty

    def find(self, obj, name=None, module=None, globs=None, extraglobs=None):
        names = _namespace_of(obj)
        if name is None:
            name = _module_name_of(obj)
        if globs is None:
            globs = names
        globs = _copy_dict(globs)
        if extraglobs is not None:
            for key in extraglobs:
                globs[key] = extraglobs[key]
        found = []
        self._add(found, name, _docstring_of_module(names), globs, name)
        self._from_test_dict(found, names, globs, name)
        self._from_members(found, names, globs, name)
        return found

    def _add(self, found, label, text, globs, filename):
        if text is None or not isinstance(text, str):
            return
        examples = self._parser.get_examples(text, label)
        if len(examples) == 0 and self._exclude_empty:
            return
        found.append(DocTest(examples, globs, label, filename, 0, text))

    def _from_test_dict(self, found, names, globs, module_name):
        table = names.get('__test__')
        if table is None:
            return
        for key in _sorted_names(table):
            value = table[key]
            label = module_name + '.__test__.' + str(key)
            if isinstance(value, str):
                self._add(found, label, value, globs, module_name)
            else:
                self._add(found, label, getattr(value, '__doc__', None),
                          globs, module_name)

    def _from_members(self, found, names, globs, module_name):
        for key in _sorted_names(names):
            if key[:1] == '#' or key == '__test__':
                continue
            value = names[key]
            if getattr(value, '__module__', None) != module_name:
                continue
            label = module_name + '.' + key
            self._add(found, label, getattr(value, '__doc__', None), globs,
                      module_name)
            if not self._recurse:
                continue
            methods = _own_methods(value, module_name)
            for method_name in methods:
                method = getattr(value, method_name, None)
                self._add(found, label + '.' + method_name,
                          getattr(method, '__doc__', None), globs, module_name)


def _own_methods(value, module_name):
    """The methods a class of this module defines itself, if it is a class."""
    outcome = __call_outcome(lambda: __class_methods(value))
    if not outcome[0] or outcome[1] is None:
        return []
    kept = []
    for name in outcome[1]:
        method = getattr(value, name, None)
        if getattr(method, '__module__', None) == module_name:
            kept.append(name)
    return kept


def _sorted_names(table):
    names = []
    for key in table:
        names.append(key)
    return sorted(names)


def _namespace_of(obj):
    if obj is None or obj == '__main__':
        return __program_namespace()
    if isinstance(obj, str):
        obj = __load_module(obj)
    if isinstance(obj, dict):
        return obj
    return __program_namespace(obj)


def _module_name_of(obj):
    if obj is None:
        names = __program_namespace()
        name = names.get('__name__')
        if name is None:
            return '__main__'
        return name
    if isinstance(obj, str):
        return obj
    name = getattr(obj, '__name__', None)
    if name is None:
        return '__main__'
    return name


def _docstring_of_module(names):
    return names.get('__doc__')


# ---------------------------------------------------------------- running

def _first_word(text):
    index = 0
    while index < len(text):
        char = text[index]
        if not (char == '_' or char.isalnum()):
            break
        index += 1
    return text[:index]


def _has_assignment(text):
    """Whether a top-level `=` makes this source an assignment rather than
    an expression. Quoted text, comments and bracketed text are passed by."""
    depth = 0
    index = 0
    total = len(text)
    quote = ''
    while index < total:
        char = text[index]
        if quote != '':
            if char == '\\':
                index += 2
                continue
            if text[index:index + len(quote)] == quote:
                index += len(quote)
                quote = ''
                continue
            index += 1
            continue
        if char == '"' or char == "'":
            if text[index:index + 3] == char * 3:
                quote = char * 3
                index += 3
            else:
                quote = char
                index += 1
            continue
        if char == '#':
            while index < total and text[index] != '\n':
                index += 1
            continue
        if char in _OPEN_BRACKETS:
            depth += 1
            index += 1
            continue
        if char in _CLOSE_BRACKETS:
            depth -= 1
            index += 1
            continue
        if char == '=' and depth <= 0:
            after = ''
            if index + 1 < total:
                after = text[index + 1]
            before = ''
            if index > 0:
                before = text[index - 1]
            if after == '=':
                index += 2                      # ==
                continue
            if before == '=' or before == '!' or before == ':':
                index += 1                      # ==, !=, :=
                continue
            if before == '<' or before == '>':
                earlier = ''
                if index > 1:
                    earlier = text[index - 2]
                if earlier == before:
                    return True                 # <<= or >>=
                index += 1                      # <= or >=
                continue
            return True                         # = or an augmented form
        if char == ';' and depth <= 0:
            return True                         # more than one statement
        index += 1
    return False


def _is_expression(source):
    body = source.strip()
    if body == '':
        return False
    if body[0] == '@':
        return False
    if _first_word(body) in _STATEMENT_WORDS:
        return False
    return not _has_assignment(body)


def _exception_detail(error, message):
    """The last line of a traceback: the kind and, after it, the message."""
    if isinstance(error, BaseException):
        detail = error.__class__.__name__
        text = str(error)
        if text != '':
            detail = detail + ': ' + text
        return detail
    return str(message)


class _Attempt:
    """One example, ready to be run under a guard that catches whatever
    stops it. An expression keeps its value, to be shown afterwards."""

    def __init__(self, source, globs):
        self.source = source
        self.globs = globs
        self.expression = _is_expression(source)
        self.value = None

    def call(self):
        # A `break` or `continue` written outside a loop belongs to no loop,
        # and a reader that does not refuse it lets it out of the example and
        # into whatever loop is running the examples. This loop, which runs
        # once whatever happens, is where such a jump lands instead, so a
        # stray one spoils its own example and no other.
        rounds = 0
        while rounds < 1:
            rounds += 1
            if self.expression:
                self.value = eval(self.source, self.globs)
            else:
                exec(self.source, self.globs)
            break


def _run_example(source, globs):
    """Run one example. Returns what it printed, and how it stopped."""
    import io
    buffer = io.StringIO()
    saved = sys.stdout
    attempt = _Attempt(source, globs)
    sys.stdout = buffer
    outcome = __call_outcome(attempt.call)
    sys.stdout = saved
    got = buffer.getvalue()
    if outcome[0]:
        if attempt.value is not None:
            got = got + repr(attempt.value) + '\n'
        return (got, None, None)
    return (got, outcome[1], outcome[2])


# ---------------------------------------------------------------- checking

def _wants_exception(want):
    head = want.lstrip()
    for header in _TRACEBACK_HEADERS:
        if head[:len(header)] == header:
            return True
    return False


def _wanted_detail(want):
    """The kind and message a traceback in the expected output asks for."""
    lines = want.split('\n')
    rest = lines[1:]
    index = 0
    while index < len(rest):
        line = rest[index]
        if line.strip() == '' or line[:1] == ' ' or line[:1] == '\t':
            index += 1
            continue
        break
    kept = []
    while index < len(rest):
        if rest[index].strip() == '':
            index += 1
            continue
        kept.append(rest[index])
        index += 1
    return '\n'.join(kept)


def _split_detail(detail):
    index = _find_from(detail, ':', 0, len(detail))
    if index < 0:
        return (detail.strip(), '')
    return (detail[:index].strip(), detail[index + 1:].strip())


def _plain_kind(kind):
    """`re.error` and `error` name the same kind; keep the last part."""
    index = len(kind) - 1
    while index >= 0:
        if kind[index] == '.':
            return kind[index + 1:]
        index -= 1
    return kind


def _ellipsis_match(want, got):
    pieces = want.split(ELLIPSIS_MARKER)
    if len(pieces) == 1:
        return want == got
    first = pieces[0]
    last = pieces[len(pieces) - 1]
    start = 0
    end = len(got)
    if first != '':
        if got[:len(first)] != first:
            return False
        start = len(first)
    if last != '':
        if len(got) < len(last) or got[len(got) - len(last):] != last:
            return False
        end = len(got) - len(last)
    if start > end:
        return False
    index = 1
    while index < len(pieces) - 1:
        piece = pieces[index]
        if piece != '':
            at = _find_from(got, piece, start, end)
            if at < 0:
                return False
            start = at + len(piece)
        index += 1
    return True


def _drop_blankline_markers(want):
    lines = want.split('\n')
    kept = []
    for line in lines:
        if line.strip() == BLANKLINE_MARKER:
            kept.append('')
        else:
            kept.append(line)
    return '\n'.join(kept)


def _squeeze(text):
    return ' '.join(text.split())


class OutputChecker:
    def check_output(self, want, got, optionflags):
        if want == got:
            return True
        if not (optionflags & DONT_ACCEPT_TRUE_FOR_1):
            if (got == '1\n' and want == 'True\n') or (got == 'True\n' and want == '1\n'):
                return True
            if (got == '0\n' and want == 'False\n') or (got == 'False\n' and want == '0\n'):
                return True
        if not (optionflags & DONT_ACCEPT_BLANKLINE):
            want = _drop_blankline_markers(want)
            if want == got:
                return True
        if optionflags & NORMALIZE_WHITESPACE:
            want = _squeeze(want)
            got = _squeeze(got)
            if want == got:
                return True
        if optionflags & ELLIPSIS:
            return _ellipsis_match(want, got)
        return want == got

    def output_difference(self, example, got, optionflags):
        return _difference(example, got)


def _indented(text, prefix='    '):
    if text == '':
        return prefix + '(nothing)\n'
    lines = text.split('\n')
    if len(lines) > 0 and lines[len(lines) - 1] == '':
        lines = lines[:len(lines) - 1]
    out = ''
    for line in lines:
        out = out + prefix + line + '\n'
    return out


def _difference(example, got):
    text = 'Failed example:\n' + _indented(example.source)
    if example.want == '':
        text = text + 'Expected nothing\n'
    else:
        text = text + 'Expected:\n' + _indented(example.want)
    if got == '':
        text = text + 'Got nothing\n'
    else:
        text = text + 'Got:\n' + _indented(got)
    return text


class DocTestRunner:
    """Runs the examples of a DocTest and counts what happened."""

    DIVIDER = '*' * 70

    def __init__(self, checker=None, verbose=None, optionflags=0):
        if checker is None:
            checker = OutputChecker()
        self._checker = checker
        self._verbose = verbose
        self.optionflags = optionflags
        self.original_optionflags = optionflags
        self.tries = 0
        self.failures = 0
        self.skips = 0
        self.reports = []

    def run(self, test, compileflags=None, out=None, clear_globs=True):
        globs = _copy_dict(test.globs)
        failures = 0
        tries = 0
        for example in test.examples:
            flags = self.optionflags
            for flag in example.options:
                if example.options[flag]:
                    flags = flags | flag
                else:
                    flags = flags & ~flag
            if flags & SKIP:
                self.skips += 1
                continue
            tries += 1
            report = self._check(test, example, globs, flags)
            if report is not None:
                failures += 1
                self.reports.append(report)
                if flags & FAIL_FAST:
                    break
        self.tries += tries
        self.failures += failures
        return TestResults(failures, tries)

    def _check(self, test, example, globs, flags):
        got, error, message = _run_example(example.source, globs)
        where = 'File "' + str(test.filename) + '", line ' + \
                str(example.lineno) + ', in ' + test.name + '\n'
        if _wants_exception(example.want):
            if error is None:
                return where + _difference(example, got) + \
                    'No exception was raised.\n'
            detail = _exception_detail(error, message)
            wanted = _wanted_detail(example.want)
            if flags & IGNORE_EXCEPTION_DETAIL:
                got_kind = _plain_kind(_split_detail(detail)[0])
                want_kind = _plain_kind(_split_detail(wanted)[0])
                if got_kind == want_kind:
                    return None
            elif self._checker.check_output(wanted + '\n', detail + '\n', flags):
                return None
            return where + 'Failed example:\n' + _indented(example.source) + \
                'Expected the exception:\n' + _indented(wanted) + \
                'Got the exception:\n' + _indented(detail)
        if error is not None:
            detail = _exception_detail(error, message)
            return where + 'Failed example:\n' + _indented(example.source) + \
                'Expected:\n' + _indented(example.want) + \
                'Got an unexpected exception:\n' + _indented(detail)
        if self._checker.check_output(example.want, got, flags):
            return None
        return where + _difference(example, got)

    def summarize(self, verbose=None):
        for report in self.reports:
            print(self.DIVIDER)
            print(report, end='')
        return TestResults(self.failures, self.tries)


# ---------------------------------------------------------------- unittest

class _NamedCase(unittest.TestCase):
    """A test the runner names by hand rather than by method name.

    The runner of this library reports a test under its `_method`, and the
    name wanted here is the name of a docstring, which is no method name, so
    the test is run from here instead of being looked up by name."""

    _label = 'runTest'

    def id(self):
        return self._label

    def run(self, result=None):
        if result is None:
            result = unittest.TestResult()
        self._result = result
        result.testsRun += 1
        outcome = __call_outcome(self.runTest)
        self.doCleanups()
        if outcome[0]:
            return result
        error = outcome[1]
        entry = [self._label, getattr(error, 'message', outcome[2]), self.id()]
        if isinstance(error, unittest.SkipTest):
            result.skipped = [*result.skipped, entry]
        elif isinstance(error, self.failureException):
            result.failures = [*result.failures, entry]
        else:
            result.errors = [*result.errors, entry]
        return result


class DocTestCase(_NamedCase):
    """The examples of one docstring, run as a single unittest test."""

    def __init__(self, test, optionflags=0, setUp=None, tearDown=None,
                 checker=None):
        unittest.TestCase.__init__(self, 'runTest')
        self._dt_test = test
        self._label = test.name
        self._dt_optionflags = optionflags
        self._dt_setUp = setUp
        self._dt_tearDown = tearDown
        self._dt_checker = checker
        self._method = test.name

    def shortDescription(self):
        return 'Doctest: ' + self._dt_test.name

    def runTest(self):
        if self._dt_setUp is not None:
            self._dt_setUp(self._dt_test)
        try:
            runner = DocTestRunner(self._dt_checker, False,
                                   self._dt_optionflags | _unittest_reportflags)
            result = runner.run(self._dt_test)
        finally:
            if self._dt_tearDown is not None:
                self._dt_tearDown(self._dt_test)
        if result.failed == 0:
            return None
        head = (str(result.failed) + ' of ' + str(result.attempted) +
                ' examples failed in ' + self._dt_test.name + '\n')
        body = ''
        for report in runner.reports:
            body = body + '\n' + '-' * 70 + '\n' + report
        raise self.failureException(head + body)

    def __repr__(self):
        return '<DocTestCase ' + self._dt_test.name + '>'


def DocTestSuite(module=None, globs=None, extraglobs=None, test_finder=None,
                 **options):
    """A unittest suite of the examples found in a module's docstrings."""
    if test_finder is None:
        test_finder = DocTestFinder()
    tests = test_finder.find(module, None, None, globs, extraglobs)
    suite = unittest.TestSuite()
    optionflags = options.get('optionflags', 0)
    setUp = options.get('setUp')
    tearDown = options.get('tearDown')
    checker = options.get('checker')
    for test in tests:
        if len(test.examples) == 0:
            continue
        suite.addTest(DocTestCase(test, optionflags, setUp, tearDown, checker))
    return suite


def DocFileSuite(*paths, **options):
    """CPython reads the named files from disk; there is no file reading
    here, so say so rather than pretend the examples passed."""
    raise NotImplementedError(
        'doctest.DocFileSuite needs to read ' + ', '.join([str(p) for p in paths]) +
        ' from disk, which this library cannot do')


def DocFileTest(path, **options):
    return DocFileSuite(path, **options)


def testfile(filename, module_relative=True, name=None, package=None,
             globs=None, verbose=None, report=True, optionflags=0,
             extraglobs=None, raise_on_error=False, parser=None,
             encoding=None):
    raise NotImplementedError(
        'doctest.testfile needs to read ' + str(filename) +
        ' from disk, which this library cannot do')


def testmod(m=None, name=None, globs=None, verbose=None, report=True,
            optionflags=0, extraglobs=None, raise_on_error=False,
            exclude_empty=False):
    """Run every example in a module and report how many failed."""
    finder = DocTestFinder(verbose, None, True, exclude_empty)
    runner = DocTestRunner(None, verbose, optionflags)
    for test in finder.find(m, name, None, globs, extraglobs):
        runner.run(test)
    if report:
        runner.summarize()
    return TestResults(runner.failures, runner.tries)


def run_docstring_examples(f, globs, verbose=False, name='NoName',
                           compileflags=None, optionflags=0):
    parser = DocTestParser()
    text = getattr(f, '__doc__', None)
    if text is None or not isinstance(text, str):
        return TestResults(0, 0)
    test = DocTest(parser.get_examples(text, name), globs, name, name, 0, text)
    runner = DocTestRunner(None, verbose, optionflags)
    runner.run(test)
    runner.summarize()
    return TestResults(runner.failures, runner.tries)


def testsource(module, name):
    for test in DocTestFinder(False, None, True, False).find(module):
        if test.name == name:
            source = ''
            for example in test.examples:
                source = source + example.source + example.want
            return source
    return ''


# The loader of this library gathers the test cases a module declares but not
# the `load_tests` it offers, and a module whose tests are all examples would
# then run none of them. Completing the loader the way CPython's does, once,
# when this module is first imported, is what lets a file's examples run.

def _module_load_tests(module):
    outcome = __call_outcome(lambda: _namespace_of(module))
    if not outcome[0]:
        return None
    names = outcome[1]
    if not isinstance(names, dict):
        return None
    return names.get('load_tests')


def _complete_loader():
    original = unittest.TestLoader.loadTestsFromModule
    if getattr(original, '_honours_load_tests', False):
        return None

    def loadTestsFromModule(self, module=None, pattern=None):
        tests = original(self, module, pattern)
        hook = _module_load_tests(module)
        if hook is None:
            return tests
        outcome = __call_outcome(lambda: hook(self, tests, pattern))
        if outcome[0]:
            if outcome[1] is None:
                return tests
            return outcome[1]
        return unittest.TestSuite([_FailedLoad(str(outcome[2]))])

    loadTestsFromModule._honours_load_tests = True
    unittest.TestLoader.loadTestsFromModule = loadTestsFromModule
    return None


class _FailedLoad(_NamedCase):
    """Stands for a `load_tests` that stopped, so the run reports it."""

    def __init__(self, message):
        unittest.TestCase.__init__(self, 'runTest')
        self._message = message
        self._label = 'load_tests'
        self._method = 'load_tests'

    def runTest(self):
        raise Exception('load_tests failed: ' + self._message)


_complete_loader()
