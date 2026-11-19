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
# Host primitives used by class bodies need non-private module bindings.
_host_call_outcome = __call_outcome


import sys
import unittest
import types


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


master = None

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

    def __init__(self, failed, attempted, *, skipped=0):
        self.failed = failed
        self.attempted = attempted
        self.skipped = skipped

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
        # An empty source is one newline. Empty expected text stays empty.
        if not source.endswith('\n'):
            source = source + '\n'
        if want != '' and not want.endswith('\n'):
            want = want + '\n'
        if exc_msg is not None and not exc_msg.endswith('\n'):
            exc_msg = exc_msg + '\n'
        self.source = source
        self.want = want
        self.exc_msg = exc_msg
        self.lineno = lineno
        self.indent = indent
        if options is None:
            options = {}
        self.options = options

    def __eq__(self, other):
        if type(self) is not type(other):
            return NotImplemented
        return (self.source == other.source and self.want == other.want and
                self.lineno == other.lineno and self.indent == other.indent and
                self.options == other.options and self.exc_msg == other.exc_msg)

    def __ne__(self, other):
        equal = self.__eq__(other)
        if equal is NotImplemented:
            return NotImplemented
        return not equal

    def __hash__(self):
        return hash((self.source, self.want, self.lineno, self.indent, self.exc_msg))


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
        count = len(self.examples)
        if count == 0:
            examples = 'no examples'
        elif count == 1:
            examples = '1 example'
        else:
            examples = str(count) + ' examples'
        return ('<DocTest ' + self.name + ' from ' + str(self.filename) + ':' +
                str(self.lineno) + ' (' + examples + ')>')

    def __eq__(self, other):
        if type(self) is not type(other):
            return NotImplemented
        return (self.examples == other.examples and self.docstring == other.docstring and
                self.globs == other.globs and self.name == other.name and
                self.filename == other.filename and self.lineno == other.lineno)

    def __ne__(self, other):
        equal = self.__eq__(other)
        if equal is NotImplemented:
            return NotImplemented
        return not equal

    def __hash__(self):
        return hash((self.docstring, self.name, self.filename, self.lineno))

    def __lt__(self, other):
        if not isinstance(other, DocTest):
            return NotImplemented
        self_line = -1 if self.lineno is None else self.lineno
        other_line = -1 if other.lineno is None else other.lineno
        return ((self.name, self.filename, self_line, id(self))
                < (other.name, other.filename, other_line, id(other)))


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


def _min_indent(text):
    """The least indentation of a non-blank line, in spaces."""
    least = None
    for line in text.split('\n'):
        if line.strip() == '':
            continue
        count = _leading_spaces(line)
        if least is None or count < least:
            least = count
    if least is None:
        return 0
    return least


def _directive_options(source, name='<string>', lineno=0):
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
        # CPython: #\s*doctest:\s*([^\n'"]*)$ — quotes or other text kill the match.
        if comment[:at_word].strip() != '':
            continue
        tail = comment[at_word + 8:]
        if "'" in tail or '"' in tail:
            continue
        for word in tail.replace(',', ' ').split():
            sign = word[:1]
            flag = None
            if sign == '+' or sign == '-':
                flag = OPTIONFLAGS_BY_NAME.get(word[1:])
            if flag is None:
                raise ValueError('line %r of the doctest for %s has an invalid option: %r' %
                                 (lineno + 1, name, word))
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
        """Examples and the text between them. Example line numbers are 0-based."""
        string = string.expandtabs()
        indent_floor = _min_indent(string)
        if indent_floor:
            stripped_lines = []
            for line in string.split('\n'):
                stripped_lines.append(line[indent_floor:])
            string = '\n'.join(stripped_lines)
        lines = string.split('\n')
        output = []
        total = len(lines)
        index = 0
        last = 0
        while index < total:
            line = lines[index]
            stripped = line.lstrip()
            if stripped[:3] == '>>>' and not _is_prompt(stripped, PS1):
                raise ValueError('line %r of the docstring for %s lacks blank after >>>: %r' %
                                 (index + 1, name, line))
            if not _is_prompt(stripped, PS1):
                index += 1
                continue
            if index > last:
                output.append('\n'.join(lines[last:index]) + '\n')
            else:
                output.append('')
            indent = len(line) - len(stripped)
            lineno = index
            source_lines = [_after_prompt(stripped)]
            index += 1
            while index < total:
                current = lines[index]
                rest = current[indent:] if len(current) >= indent else ''
                if rest[:3] == '...':
                    if not _is_prompt(rest, PS2):
                        raise ValueError('line %r of the docstring for %s lacks blank after ...: %r' %
                                         (index + 1, name, current))
                    source_lines.append(_after_prompt(rest))
                    index += 1
                    continue
                lead = current.lstrip()
                if lead[:3] == '...':
                    if not _is_prompt(lead, PS2):
                        raise ValueError('line %r of the docstring for %s lacks blank after ...: %r' %
                                         (index + 1, name, current))
                    # Numbered from the example's first line, as the prompt check is.
                    raise ValueError('line %r of the docstring for %s has inconsistent leading whitespace: %r' %
                                     (lineno + len(source_lines), name, current))
                break
            want_lines = []
            prefix = ' ' * indent
            while index < total:
                current = lines[index]
                # A prompt at any indent starts the next example. CPython's
                # want pattern stops at ^[ ]*>>>; it is not a want line.
                if current.strip() == '' or _is_prompt(current.lstrip(), PS1):
                    break
                if not current.startswith(prefix):
                    raise ValueError('line %r of the docstring for %s has inconsistent leading whitespace: %r' %
                                     (index + 1, name, current))
                want_lines.append(current[indent:])
                index += 1
            source = '\n'.join(source_lines)
            options = _directive_options(source, name, lineno)
            if all(part.strip() == '' or part.lstrip().startswith('#') for part in source_lines):
                if options:
                    raise ValueError('line %r of the doctest for %s has an option '
                                     'directive on a line with no example: %r' %
                                     (lineno, name, source))
                last = index
                continue
            want = ''
            if len(want_lines) > 0:
                want = '\n'.join(want_lines) + '\n'
            output.append(Example(source, want, None, lineno, indent, options))
            last = index
        output.append('\n'.join(lines[last:]))
        return output

    def get_examples(self, string, name='<string>'):
        examples = []
        for piece in self.parse(string, name):
            if isinstance(piece, Example):
                examples.append(piece)
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
        self._file_name, self._source_lines = _module_file_and_lines(obj, module)
        # A class dict does not contain the names the examples use.
        if globs is None and module is not None and hasattr(module, '__dict__'):
            globs = module.__dict__
        if callable(obj) and not isinstance(obj, type):
            if name is None:
                name = getattr(obj, "__name__", "<callable>")
            if globs is None:
                globs = names
            globs = _copy_dict(globs)
            globs[getattr(obj, "__name__", name)] = obj
            if extraglobs is not None:
                globs.update(extraglobs)
            found = []
            self._add(found, name, getattr(obj, "__doc__", None), globs, name, obj)
            return found
        if name is None:
            name = _module_name_of(obj)
        if globs is None:
            globs = names
        globs = _copy_dict(globs)
        if extraglobs is not None:
            for key in extraglobs:
                globs[key] = extraglobs[key]
        found = []
        self._add(found, name, _docstring_of_module(names), globs, name, obj)
        self._from_test_dict(found, names, globs, name)
        self._from_members(found, names, globs, name)
        return found

    def _add(self, found, label, text, globs, filename, owner=None):
        if text is None or not isinstance(text, str):
            return
        examples = self._parser.get_examples(text, label)
        if len(examples) == 0 and self._exclude_empty:
            return
        if getattr(self, '_file_name', None):
            filename = self._file_name
        lineno = _docstring_lineno(owner, getattr(self, '_source_lines', None))
        found.append(DocTest(examples, globs, label, filename, lineno, text))

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
            if not isinstance(value, (type, types.FunctionType, types.BuiltinFunctionType, types.MethodType)):
                continue
            if getattr(value, '__module__', None) != module_name:
                continue
            label = module_name + '.' + key
            self._add(found, label, getattr(value, '__doc__', None), globs,
                      module_name, value)
            if not self._recurse:
                continue
            methods = _own_methods(value, module_name)
            for method_name in methods:
                method = getattr(value, method_name, None)
                self._add(found, label + '.' + method_name,
                          getattr(method, '__doc__', None), globs, module_name, method)


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
    return getattr(obj, "__globals__", getattr(obj, "__dict__", {}))


def _module_name_of(obj):
    if isinstance(obj, dict):
        return obj.get("__name__", "__main__")
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

def _exception_detail(error, message, namespace=None):
    """The last line of a traceback: the kind and, after it, the message.

    The name is the exception class's own, read from its metadata; nothing
    about the namespace the example ran in changes it."""
    if isinstance(error, BaseException):
        kind = error.__class__
        detail = kind.__name__
        module = kind.__module__
        if module != 'builtins' and module != '__main__':
            detail = module + '.' + detail
        text = str(error)
        if isinstance(error, SyntaxError):
            text = str(error.msg or '<no detail available>')
            if error.lineno is None and error.filename is not None:
                text = text + ' (' + str(error.filename) + ')'
        if text != '':
            detail = detail + ': ' + text
        return detail
    return str(message)


def _format_traceback(error):
    """Traceback text for a failure report. The last line names the exception."""
    detail = _exception_detail(error, error)
    if detail == '' or detail[-1:] != '\n':
        detail = detail + '\n'
    try:
        import traceback
        if isinstance(error, BaseException):
            text = ''.join(traceback.format_exception(type(error), error, getattr(error, '__traceback__', None)))
            if isinstance(text, str) and text.startswith('Traceback'):
                if text[-1:] != '\n':
                    text = text + '\n'
                return text
    except Exception:
        pass
    # A frame line lets an ellipsis in the expected report match the stack.
    return 'Traceback (most recent call last):\n  File "<doctest>", line 1, in <module>\n' + detail


class _Attempt:
    """Run one example with interactive expression display."""

    def __init__(self, source, globs, filename):
        self.source = source
        self.globs = globs
        self.filename = filename

    def call(self):
        # A `break` or `continue` written outside a loop belongs to no loop,
        # and a reader that does not refuse it lets it out of the example and
        # into whatever loop is running the examples. This loop, which runs
        # once whatever happens, is where such a jump lands instead, so a
        # stray one spoils its own example and no other.
        rounds = 0
        while rounds < 1:
            rounds += 1
            exec(compile(self.source, self.filename, 'single'), self.globs)
            break


def _run_example(source, globs, buffer, filename):
    """Run one example. Returns what it printed, and how it stopped."""
    attempt = _Attempt(source, globs, filename)
    outcome = __call_outcome(attempt.call)
    got = buffer.getvalue()
    buffer.seek(0)
    buffer.truncate(0)
    if outcome[0]:
        if got and not got.endswith('\n'):
            got += '\n'
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
        if line[:1].isalnum() or line[:1] == '_':
            break
        index += 1
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
    """A <BLANKLINE> marker is only a blank line when it starts the line."""
    lines = want.split('\n')
    kept = []
    marker = BLANKLINE_MARKER
    width = len(marker)
    for line in lines:
        if line.startswith(marker) and line[width:].strip(' ') == '':
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
        return _output_difference(example, got, optionflags)


def _indented(text, prefix='    '):
    """Indent non-blank lines. A blank line stays blank, as CPython's _indent."""
    if text == '':
        return prefix + '(nothing)\n'
    lines = text.split('\n')
    if len(lines) > 0 and lines[len(lines) - 1] == '':
        lines = lines[:len(lines) - 1]
    out = []
    for line in lines:
        if line:
            out.append(prefix + line)
        else:
            out.append('')
    return '\n'.join(out) + '\n'


def _mark_blanklines(text):
    """Turn space-only lines into <BLANKLINE>, leaving the newline."""
    if '\n' not in text:
        return text
    parts = text.split('\n')
    index = 0
    last = len(parts) - 1
    while index < last:
        if parts[index].strip(' ') == '':
            parts[index] = BLANKLINE_MARKER
        index += 1
    return '\n'.join(parts)


def _fancy_diff(want, got, optionflags):
    """CPython's udiff, cdiff, or ndiff report, or None to use Expected/Got."""
    if not optionflags & (REPORT_UDIFF | REPORT_CDIFF | REPORT_NDIFF):
        return None
    if not (optionflags & REPORT_NDIFF):
        if want.count('\n') <= 2 or got.count('\n') <= 2:
            return None
    import difflib
    want_lines = want.splitlines(True)
    got_lines = got.splitlines(True)
    if optionflags & REPORT_UDIFF:
        diff = list(difflib.unified_diff(want_lines, got_lines, n=2))[2:]
        kind = 'unified diff with -expected +actual'
    elif optionflags & REPORT_CDIFF:
        diff = list(difflib.context_diff(want_lines, got_lines, n=2))[2:]
        kind = 'context diff with expected followed by actual'
    else:
        engine = difflib.Differ(charjunk=difflib.IS_CHARACTER_JUNK)
        diff = list(engine.compare(want_lines, got_lines))
        kind = 'ndiff with -expected +actual'
    return 'Differences (%s):\n' % kind + _indented(''.join(diff))


def _output_difference(example, got, optionflags):
    want = example.want
    if not (optionflags & DONT_ACCEPT_BLANKLINE):
        got = _mark_blanklines(got)
    fancy = _fancy_diff(want, got, optionflags)
    if fancy is not None:
        return fancy
    if want == '':
        text = 'Expected nothing\n'
    else:
        text = 'Expected:\n' + _indented(want)
    if got == '':
        text = text + 'Got nothing\n'
    else:
        text = text + 'Got:\n' + _indented(got)
    return text


def _difference(example, got, optionflags=0):
    return 'Failed example:\n' + _indented(example.source) + _output_difference(example, got, optionflags)


class DocTestRunner:
    """Runs the examples of a DocTest and counts what happened."""

    DIVIDER = '*' * 70

    def __init__(self, checker=None, verbose=None, optionflags=0):
        if checker is None:
            checker = OutputChecker()
        self._checker = checker
        # Unspecified verbosity follows -v, read when the runner is built.
        if verbose is None:
            verbose = '-v' in sys.argv
        self._verbose = verbose
        self.optionflags = optionflags
        self.original_optionflags = optionflags
        self.tries = 0
        self.failures = 0
        self.skips = 0
        self.reports = []

    def report_start(self, out, test, example):
        if not self._verbose:
            return
        if example.want:
            out('Trying:\n' + _indented(example.source) +
                'Expecting:\n' + _indented(example.want))
        else:
            out('Trying:\n' + _indented(example.source) +
                'Expecting nothing\n')

    def _line_number(self, test, example):
        # example.lineno is the 0-based line of the prompt inside the
        # docstring. test.lineno is the 0-based file line of that docstring.
        # The line a failure names counts both, including the prompt line.
        if test.lineno is not None and example.lineno is not None:
            return test.lineno + example.lineno + 1
        if example.lineno is not None:
            return example.lineno + 1
        return '?'

    def run(self, test, compileflags=None, out=None, clear_globs=True):
        globs = _copy_dict(test.globs)
        failures = 0
        tries = 0
        # One stream stands in for sys.stdout through a whole run, the way
        # the reference runner keeps a single stand-in and drains it after
        # each example: an object an early example binds out of sys.stdout
        # then keeps writing where the later examples are read from.
        import io
        buffer = io.StringIO()
        saved = sys.stdout
        if out is None:
            out = saved.write
        sys.stdout = buffer
        import linecache
        self.test = test
        self._saved_getlines = linecache.getlines
        linecache.getlines = self._example_lines
        original = self.optionflags
        try:
            for index, example in enumerate(test.examples):
                filename = '<doctest %s[%d]>' % (test.name, index)
                flags = original
                for flag in example.options:
                    if example.options[flag]:
                        flags = flags | flag
                    else:
                        flags = flags & ~flag
                quiet = (flags & REPORT_ONLY_FIRST_FAILURE) and failures > 0
                if flags & SKIP:
                    self.skips += 1
                    continue
                tries += 1
                if not quiet:
                    self.report_start(out, test, example)
                report = self._check(test, example, globs, flags, buffer, filename)
                if report is None:
                    if not quiet and self._verbose:
                        out('ok\n')
                    continue
                failures += 1
                self.reports.append(report)
                if not quiet:
                    out(report)
                if flags & FAIL_FAST:
                    break
        finally:
            linecache.getlines = self._saved_getlines
            sys.stdout = saved
            self.optionflags = original
        self.tries += tries
        self.failures += failures
        return TestResults(failures, tries)

    def _example_lines(self, filename, module_globals=None):
        """Source of the example compiled as <doctest name[n]>, else the real file."""
        if (self.test is not None and isinstance(filename, str) and
                filename.startswith('<doctest ') and filename.endswith(']>')):
            body = filename[len('<doctest '):-2]
            bracket = body.rfind('[')
            if bracket >= 0 and body[:bracket] == self.test.name:
                number = body[bracket + 1:]
                if number.isdigit():
                    index = int(number)
                    if 0 <= index < len(self.test.examples):
                        return self.test.examples[index].source.splitlines(True)
        return self._saved_getlines(filename, module_globals)

    def _check(self, test, example, globs, flags, buffer, filename):
        got, error, message = _run_example(example.source, globs, buffer, filename)
        header = self.DIVIDER + '\nFile "' + str(test.filename) + '", line ' + \
            str(self._line_number(test, example)) + ', in ' + test.name + '\n'
        if _wants_exception(example.want):
            if error is None:
                return header + _difference(example, got, flags) + \
                    'No exception was raised.\n'
            detail = _exception_detail(error, message, globs)
            wanted = _wanted_detail(example.want)
            if flags & IGNORE_EXCEPTION_DETAIL:
                got_kind = _plain_kind(_split_detail(detail)[0])
                want_kind = _plain_kind(_split_detail(wanted)[0])
                if got_kind == want_kind:
                    return None
            elif self._checker.check_output(wanted + '\n', detail + '\n', flags):
                return None
            return header + _difference(example, got + _format_traceback(error), flags)
        if error is not None:
            return header + 'Failed example:\n' + _indented(example.source) + \
                'Exception raised:\n' + _indented(_format_traceback(error))
        if self._checker.check_output(example.want, got, flags):
            return None
        return header + _difference(example, got, flags)

    def merge(self, other):
        self.failures += other.failures
        self.tries += other.tries
        self.skips += other.skips
        self.reports.extend(other.reports)

    def summarize(self, verbose=None):
        # Failure text was written as each example finished. The summary is
        # the counts; printing the reports again would show every failure twice.
        return TestResults(self.failures, self.tries)


# ---------------------------------------------------------------- unittest

class _NamedCase(unittest.TestCase):
    """Preserve docstring identifiers while using the standard TestCase lifecycle."""

    _label = 'runTest'

    def id(self):
        return self._label



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

    def shortDescription(self):
        return 'Doctest: ' + self._dt_test.name

    def runTest(self):
        import io
        captured = io.StringIO()
        if self._dt_setUp is not None:
            self._dt_setUp(self._dt_test)
        try:
            runner = DocTestRunner(self._dt_checker, False,
                                   self._dt_optionflags | _unittest_reportflags)
            result = runner.run(self._dt_test, out=captured.write)
        finally:
            if self._dt_tearDown is not None:
                self._dt_tearDown(self._dt_test)
        if result.failed == 0:
            if runner.skips > 0 and result.attempted == 0:
                raise unittest.SkipTest('all examples were skipped')
            return None
        head = (str(result.failed) + ' of ' + str(result.attempted) +
                ' examples failed in ' + self._dt_test.name + '\n')
        raise self.failureException(head + captured.getvalue())

    def __repr__(self):
        name = self._dt_test.name.split('.')
        return name[-1] + ' (' + '.'.join(name[:-1]) + ')'

    def __str__(self):
        return self.__repr__()

    def __eq__(self, other):
        if type(self) is not type(other):
            return NotImplemented
        return (self._dt_test == other._dt_test and
                self._dt_optionflags == other._dt_optionflags and
                self._dt_setUp == other._dt_setUp and
                self._dt_tearDown == other._dt_tearDown and
                self._dt_checker == other._dt_checker)

    def __ne__(self, other):
        equal = self.__eq__(other)
        if equal is NotImplemented:
            return NotImplemented
        return not equal

    def __hash__(self):
        return hash((self._dt_optionflags, self._dt_setUp, self._dt_tearDown, self._dt_checker))


def _normalize_module(module, depth=2):
    if isinstance(module, types.ModuleType):
        return module
    if isinstance(module, str):
        return __load_module(module)
    if module is not None:
        raise TypeError('Expected a module, string, or None')
    try:
        try:
            name = sys._getframemodulename(depth)
        except AttributeError:
            name = sys._getframe(depth).f_globals['__name__']
        return sys.modules[name]
    except KeyError:
        return None


def DocTestSuite(module=None, globs=None, extraglobs=None, test_finder=None,
                 **options):
    """A unittest suite of the examples found in a module's docstrings."""
    if test_finder is None:
        test_finder = DocTestFinder()
    if module is None:
        caller = __program_namespace(1).f_globals
        module = sys.modules.get(sys._getframemodulename(1), caller)
    else:
        module = _normalize_module(module)
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
    if options.get('module_relative', True) and options.get('package') is None:
        options['package'] = _normalize_module(None, 2)
    suite = unittest.TestSuite()
    for path in paths:
        suite.addTest(DocFileTest(path, **options))
    return suite


def _load_testfile(filename, package, module_relative, encoding):
    import os
    if module_relative:
        if isinstance(package, str):
            package = __load_module(package)
        elif package is None:
            package = _normalize_module(None, 3)
        elif not isinstance(package, types.ModuleType):
            raise TypeError('Expected a module, string, or None')
        if filename.startswith('/'):
            raise ValueError('Module-relative files may not have absolute paths')
        filename = os.path.join(*filename.split('/'))
        if hasattr(package, '__file__'):
            basedir = os.path.dirname(package.__file__)
        elif package.__name__ == '__main__':
            basedir = os.path.dirname(sys.argv[0]) if sys.argv and sys.argv[0] else os.curdir
        else:
            for directory in getattr(package, '__path__', ()):
                candidate = os.path.join(directory, filename)
                if os.path.exists(candidate):
                    filename = candidate
                    break
            else:
                raise ValueError("Can't resolve paths relative to the module %r (it has no __file__)" % package.__name__)
            basedir = ''
        filename = os.path.join(basedir, filename)
        loader = getattr(package, '__loader__', None)
        if loader is None:
            loader = getattr(getattr(package, '__spec__', None), 'loader', None)
        if hasattr(loader, 'get_data'):
            text = loader.get_data(filename).decode(encoding)
            return text.replace('\r\n', '\n').replace('\r', '\n'), filename
    with open(filename, encoding=encoding) as source:
        return source.read(), filename


def DocFileTest(path, **options):
    import os
    package = options.get('package')
    relative = options.get('module_relative', True)
    if package is not None and not relative:
        raise ValueError('Package may only be specified for module-relative paths.')
    text, path = _load_testfile(path, package, relative,
                                options.get('encoding') or 'utf-8')
    globs = _copy_dict(options.get('globs') or {})
    globs.update(options.get('extraglobs') or {})
    globs.setdefault('__name__', '__main__')
    parser = options.get('parser') or DocTestParser()
    test = parser.get_doctest(text, globs, os.path.basename(path), path, 0)
    return DocTestCase(test, options.get('optionflags', 0), options.get('setUp'),
                       options.get('tearDown'), options.get('checker'))


def testfile(filename, module_relative=True, name=None, package=None,
             globs=None, verbose=None, report=True, optionflags=0,
             extraglobs=None, raise_on_error=False, parser=None,
             encoding=None):
    import os
    global master
    if package is not None and not module_relative:
        raise ValueError('Package may only be specified for module-relative paths.')
    text, filename = _load_testfile(filename, package, module_relative,
                                    encoding or 'utf-8')
    if name is None:
        name = os.path.basename(filename)
    namespace = _copy_dict(globs or {})
    namespace.update(extraglobs or {})
    namespace.setdefault('__name__', '__main__')
    runner_class = DebugRunner if raise_on_error else DocTestRunner
    runner = runner_class(verbose=verbose, optionflags=optionflags)
    parser = parser or DocTestParser()
    runner.run(parser.get_doctest(text, namespace, name, filename, 0))
    if report:
        runner.summarize()
    if master is None:
        master = runner
    else:
        master.merge(runner)
    return TestResults(runner.failures, runner.tries, skipped=runner.skips)


def _is_module(obj):
    return isinstance(obj, types.ModuleType)


def _module_file_and_lines(obj, module):
    """The module file name and its source lines, for failure locations."""
    import linecache
    if module is None:
        if _is_module(obj):
            module = obj
        else:
            modname = getattr(obj, '__module__', None)
            if isinstance(modname, str):
                module = sys.modules.get(modname)
    filename = None
    lines = None
    if module is None or isinstance(module, dict):
        return filename, lines
    filename = getattr(module, '__file__', None) or getattr(module, '__name__', None)
    if isinstance(filename, str) and filename.endswith('.pyc'):
        filename = filename[:-1]
    file_for_lines = getattr(module, '__file__', None)
    if not isinstance(file_for_lines, str):
        return filename, lines
    try:
        got = linecache.getlines(file_for_lines, getattr(module, '__dict__', None))
    except Exception:
        got = None
    if not got and '.zip/' in file_for_lines:
        try:
            import _tokenize
            text = _tokenize._archived_source(file_for_lines)
        except Exception:
            text = None
        if isinstance(text, str) and text != '':
            got = [line + '\n' for line in text.splitlines()]
    if got:
        lines = got
    return filename, lines


def _docstring_lineno(obj, source_lines):
    """0-based file line where obj's docstring starts. 0 when unknown.

    Module docstrings stay at 0 so their reported lines do not move.
    """
    if obj is None or not source_lines:
        return 0
    if isinstance(obj, dict) or _is_module(obj):
        return 0
    import re
    lineno = None
    if isinstance(obj, type):
        cname = getattr(obj, '__name__', None)
        if cname:
            pat = re.compile(r'^\s*class\s*' + re.escape(cname) + r'\b')
            for index, line in enumerate(source_lines):
                if pat.match(line):
                    lineno = index
                    break
    else:
        code = getattr(obj, '__code__', None)
        if code is None:
            func = getattr(obj, '__func__', None)
            code = getattr(func, '__code__', None) if func is not None else None
        first = getattr(code, 'co_firstlineno', None) if code is not None else None
        filename = getattr(code, 'co_filename', None) if code is not None else None
        if first:
            lineno = first - 1
        # <doctest name[n]> is the example that defined this function. Its
        # source is on linecache only while that example's runner is active.
        if isinstance(filename, str) and filename.startswith('<doctest ') and first:
            import linecache
            try:
                got = linecache.getlines(filename)
            except Exception:
                got = None
            if got:
                source_lines = got
            else:
                return first
    if lineno is None:
        return 0
    quote = re.compile(r'(^|.*:)\s*\w*("|\')')
    limit = len(source_lines)
    while lineno < limit:
        if quote.match(source_lines[lineno]):
            return lineno
        lineno = lineno + 1
    return 0


def testmod(m=None, name=None, globs=None, verbose=None, report=True,
            optionflags=0, extraglobs=None, raise_on_error=False,
            exclude_empty=False):
    """Run every example in a module and report how many failed."""
    if m is None:
        m = sys.modules.get('__main__')
    finder = DocTestFinder(verbose, None, True, exclude_empty)
    runner = DocTestRunner(None, verbose, optionflags)
    module = None if m is None or isinstance(m, dict) else m
    for test in finder.find(m, name, module, globs, extraglobs):
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


def _comment_line(line):
    line = line.rstrip()
    if line:
        return '# ' + line
    return '#'


def script_from_examples(s):
    """The examples as a script, and everything else as comments."""
    output = []
    for piece in DocTestParser().parse(s):
        if isinstance(piece, Example):
            output.append(piece.source[:-1])
            want = piece.want
            if want:
                output.append('# Expected:')
                for line in want.split('\n')[:-1]:
                    output.append('## ' + line)
        else:
            for line in piece.split('\n')[:-1]:
                output.append(_comment_line(line))
    while output and output[-1] == '#':
        output.pop()
    while output and output[0] == '#':
        output.pop(0)
    return '\n'.join(output) + '\n'


def testsource(module, name):
    module = _normalize_module(module)
    found = None
    for test in DocTestFinder(False, None, True, False).find(module):
        if test.name == name:
            found = test
            break
    if found is None:
        raise ValueError(name, 'not found in tests')
    return script_from_examples(found.docstring or '')


def debug_script(src, pm=False, globs=None):
    """Run a script under the debugger."""
    import pdb
    if globs:
        globs = _copy_dict(globs)
    else:
        globs = {}
    if pm:
        try:
            exec(src, globs, globs)
        except Exception:
            print(sys.exc_info()[1])
            debugger = pdb.Pdb(nosigint=True)
            debugger.reset()
            debugger.interaction(None, sys.exc_info()[2])
    else:
        pdb.Pdb(nosigint=True).run('exec(%r)' % src, globs, globs)


def debug_src(src, pm=False, globs=None):
    """Debug one docstring's examples."""
    debug_script(script_from_examples(src), pm, globs)


def debug(module, name, pm=False):
    """Debug the docstring named within a module."""
    module = _normalize_module(module)
    debug_script(testsource(module, name), pm, module.__dict__)


# Export the supported public doctest API; helper TestCase classes stay private.
__all__ = ['register_optionflag', 'DONT_ACCEPT_TRUE_FOR_1', 'DONT_ACCEPT_BLANKLINE', 'NORMALIZE_WHITESPACE', 'ELLIPSIS', 'SKIP', 'IGNORE_EXCEPTION_DETAIL', 'COMPARISON_FLAGS', 'REPORT_UDIFF', 'REPORT_CDIFF', 'REPORT_NDIFF', 'REPORT_ONLY_FIRST_FAILURE', 'REPORTING_FLAGS', 'FAIL_FAST', 'Example', 'DocTest', 'DocTestParser', 'DocTestFinder', 'DocTestRunner', 'OutputChecker', 'DocTestFailure', 'UnexpectedException', 'testmod', 'testfile', 'run_docstring_examples', 'DocTestSuite', 'DocFileSuite', 'set_unittest_reportflags', 'testsource', 'script_from_examples', 'debug_src', 'debug_script', 'debug']
