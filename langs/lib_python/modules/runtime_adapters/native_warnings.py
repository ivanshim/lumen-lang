# Filters belong to the module; each entered context keeps its former
# list, and the records are shared with the caller who asked for them.
filters = [
    ['default', '', DeprecationWarning, '__main__', 0],
    ['ignore', '', DeprecationWarning, '', 0],
    ['ignore', '', PendingDeprecationWarning, '', 0],
    ['ignore', '', ImportWarning, '', 0],
    ['ignore', '', ResourceWarning, '', 0],
]

class _State:
    def __init__(self):
        self.context = None
        self.seen = []

_state = _State()

class WarningMessage:
    _WARNING_DETAILS = ('message', 'category', 'filename', 'lineno', 'file', 'line', 'source')

    def __init__(self, message, category, filename, lineno, file=None, line=None, source=None):
        self.message = message
        self.category = category
        self.filename = filename
        self.lineno = lineno
        self.file = file
        self.line = line
        self.source = source
        self._category_name = category.__name__ if category else None

    def __str__(self):
        return ("{message : %r, category : %r, filename : %r, lineno : %s, "
                "line : %r}" % (self.message, self._category_name,
                                self.filename, self.lineno, self.line))


def _matches(pattern, text, folded=False):
    # CPython compiles a filter's message pattern case-insensitively and
    # its module pattern case-sensitively, then anchors both with
    # `match` (not `search`) against the warning's own text -- see
    # `warnings._filters_mutated`'s `re.compile(message, re.I)` and
    # `re.compile(module)`.
    if pattern == '':
        return True
    import re
    flags = re.IGNORECASE if folded else 0
    return re.compile(pattern, flags).match(text) is not None


def _check(action, category, lineno):
    if action not in ['error', 'ignore', 'always', 'default', 'once', 'module']:
        raise AssertionError('invalid action: ' + repr(action))
    # A filter's category may name several warning classes as a tuple.
    if isinstance(category, tuple):
        if not category or not all(isinstance(item(), Warning) for item in category):
            raise AssertionError('category must be a Warning subclass')
    elif not isinstance(category(), Warning):
        raise AssertionError('category must be a Warning subclass')
    if (type(lineno) != type(0) and type(lineno) != type(True)) or lineno < 0:
        raise AssertionError('lineno must be an int >= 0')


def filterwarnings(action, message='', category=Warning, module='', lineno=0, append=False):
    global filters
    _check(action, category, lineno)
    if type(message) != type('') or type(module) != type(''):
        raise AssertionError('warning filter patterns must be strings')
    _matches(message, '', True)
    _matches(module, '')
    rule = [action, message, category, module, lineno]
    if append:
        if rule not in filters:
            filters = [*filters, rule]
    else:
        filters = [rule, *[old for old in filters if old != rule]]
    _state.seen = []


def simplefilter(action, category=Warning, lineno=0, append=False):
    # Unlike filterwarnings, CPython's simplefilter also accepts tuples
    # for the exception matcher used by assertWarns.
    global filters
    if action not in ['error', 'ignore', 'always', 'default', 'once', 'module']:
        raise ValueError('invalid action: ' + repr(action))
    if not isinstance(lineno, int):
        raise TypeError('lineno must be an int')
    if lineno < 0:
        raise ValueError('lineno must be an int >= 0')
    rule = [action, '', category, '', lineno]
    if append:
        if rule not in filters:
            filters = [*filters, rule]
    else:
        filters = [rule, *[old for old in filters if old != rule]]
    _state.seen = []


def resetwarnings():
    global filters
    filters = []
    _state.seen = []


def _import_frame(frame):
    # Native loader bridges stand in for frames hidden by the import machinery.
    return frame.f_globals.get('__name__') in (
        '_imp', '_runtime_import', 'importlib._bootstrap',
        'importlib._bootstrap_external',
    )


def _location(stacklevel, prefixes=()):
    import sys
    frame = sys._getframe(2)
    hide_imports = stacklevel > 1 and not _import_frame(frame)
    for _ in range(max(stacklevel, 1) - 1):
        frame = frame.f_back
        while frame is not None and hide_imports and (
                _import_frame(frame) or
                any(frame.f_code.co_filename.startswith(prefix) for prefix in prefixes)):
            frame = frame.f_back
        if frame is None:
            return ['<sys>', 0, sys.__dict__.get('__name__', '<string>')]
    return [frame.f_code.co_filename, frame.f_lineno,
            frame.f_globals.get('__name__', '<string>')]

def warn(message, category=None, stacklevel=1, source=None, *, skip_file_prefixes=()):
    if not isinstance(stacklevel, int) and not hasattr(type(stacklevel), '__index__'):
        raise TypeError("'" + type(stacklevel).__name__ + "' object cannot be interpreted as an integer")
    import operator
    stacklevel = operator.index(stacklevel)
    if not isinstance(skip_file_prefixes, tuple):
        raise TypeError('skip_file_prefixes must be a tuple of strs.')
    for prefix in skip_file_prefixes:
        if not isinstance(prefix, str):
            raise TypeError("Found non-str '" + type(prefix).__name__ + "' in skip_file_prefixes.")
    if isinstance(message, Warning):
        category = type(message)
    elif category is None:
        category = UserWarning
    if not isinstance(category(), Warning):
        raise TypeError('category must be a Warning subclass')
    if not isinstance(message, Warning):
        message = category(message)
    if skip_file_prefixes is not None and len(skip_file_prefixes) != 0:
        place = _location(max(2, stacklevel), skip_file_prefixes)
    else:
        place = _location(stacklevel)
    filename = place[0]
    # Module filters follow the caller's globals, independently of its
    # filename and function metadata. An explicit None disables warnings;
    # absent or non-string namespace names use the anonymous-code name.
    module = place[2]
    if module is None:
        return None
    if not isinstance(module, str):
        module = '<string>'
    _warn(message, category, filename, place[1], module, source)


def warn_explicit(message, category, filename, lineno, module=None, registry=None, module_globals=None, source=None):
    if registry is not None or module_globals is not None:
        raise NotImplementedError('explicit warning registries cannot run yet')
    if module is None:
        module = filename
        if module[-3:] == '.py':
            module = module[:-3]
    if isinstance(message, Warning):
        category = type(message)
    else:
        message = category(message)
    _warn(message, category, filename, lineno, module, source)


def _warn(message, category, filename, lineno, module, source):
    action = 'default'
    for rule in filters:
        if isinstance(message, rule[2]) and _matches(rule[1], str(message), True) and _matches(rule[3], module) and (rule[4] == 0 or rule[4] == lineno):
            action = rule[0]
            break
    if action == 'ignore':
        return None
    if action == 'error':
        raise message
    if action != 'always':
        key = [str(message)]
        if action != 'once':
            key = [*key, module]
        if action == 'default':
            key = [*key, lineno]
        for prior in _state.seen:
            if prior[0] is category and prior[1] == key:
                return None
        _state.seen = [*_state.seen, [category, key]]
    context = _state.context
    if context is not None and context.record:
        context.records.append(WarningMessage(message, category, filename, lineno, source=source))
    else:
        if source is not None:
            raise NotImplementedError('warning allocation traceback cannot run yet')
        showwarning(message, category, filename, lineno)


def formatwarning(message, category, filename, lineno, line=None):
    text = filename + ':' + str(lineno) + ': ' + category.__name__ + ': ' + str(message) + '\n'
    if line is not None:
        text += '  ' + line + '\n'
    return text


def showwarning(message, category, filename, lineno, file=None, line=None):
    if file is None:
        import sys
        print(formatwarning(message, category, filename, lineno, line), end='', file=sys.stderr)
    else:
        file.write(formatwarning(message, category, filename, lineno, line))


class catch_warnings:
    def __init__(self, *, record=False, module=None, action=None, category=Warning, lineno=0, append=False, _internal=False):
        if module is not None:
            raise NotImplementedError('alternate warning modules cannot run yet')
        self.record = record
        self.action = action
        self.category = category
        self.lineno = lineno
        self.append = append
        self.entered = False
        self.records = []

    def __enter__(self):
        global filters
        if self.entered:
            raise RuntimeError('warning capture is already entered')
        self.entered = True
        self.previous = _state.context
        self.filters = filters
        filters = [*filters]
        _state.seen = []
        if self.record:
            _state.context = self
        if self.action is not None:
            simplefilter(self.action, self.category, self.lineno, self.append)
        if self.record:
            return self.records
        return None

    def __exit__(self, kind, value, traceback):
        global filters
        if not self.entered:
            raise RuntimeError('warning capture was not entered')
        _state.context = self.previous
        filters = self.filters
        _state.seen = []
        return False

# Source: CPython 3b564385e4c9, Lib/_py_warnings.py; PSF License.
import sys


_DEPRECATED_MSG = "{name!r} is deprecated and slated for removal in Python {remove}"

def _deprecated(name, message=_DEPRECATED_MSG, *, remove, _version=sys.version_info):
    """Warn that *name* is deprecated or should be removed.

    RuntimeError is raised if *remove* specifies a major/minor tuple older than
    the current Python version or the same version but past the alpha.

    The *message* argument is formatted with *name* and *remove* as a Python
    version tuple (e.g. (3, 11)).

    """
    remove_formatted = f"{remove[0]}.{remove[1]}"
    if (_version[:2] > remove) or (_version[:2] == remove and _version[3] != "alpha"):
        msg = f"{name!r} was slated for removal after Python {remove_formatted} alpha"
        raise RuntimeError(msg)
    else:
        msg = message.format(name=name, remove=remove_formatted)
        warn(msg, DeprecationWarning, stacklevel=3)


class _OptionError(Exception):
    pass


def _startup_filter(option):
    import re
    import builtins
    fields = option.split(':')
    if len(fields) > 5:
        raise _OptionError('too many fields (max 5): %r' % option)
    fields += [''] * (5 - len(fields))
    action, message, category, module, lineno = [field.strip() for field in fields]
    if not action:
        action = 'default'
    if action == 'all':
        action = 'always'
    else:
        choices = [name for name in ('default', 'always', 'ignore', 'module', 'once', 'error') if name.startswith(action)]
        if len(choices) != 1:
            raise _OptionError('invalid action: %r' % action)
        action = choices[0]
    if not category:
        category = Warning
    elif '.' not in category:
        name = category
        category = getattr(builtins, name, None)
        if category is None:
            raise _OptionError('unknown warning category: %r' % name)
    else:
        name, _, attribute = category.rpartition('.')
        try:
            category = getattr(__import__(name, None, None, [attribute]), attribute)
        except (ImportError, AttributeError):
            raise _OptionError('invalid module name: %r' % name)
    if not isinstance(category, type) or not issubclass(category, Warning):
        raise _OptionError('invalid warning category: %r' % category)
    try:
        lineno = int(lineno or '0')
        if lineno < 0:
            raise ValueError
    except ValueError:
        raise _OptionError('invalid lineno')
    message = re.escape(message)
    module = re.escape(module) + ('\\Z' if module else '')
    filterwarnings(action, message, category, module, lineno)


for _option in sys.warnoptions:
    try:
        _startup_filter(_option)
    except _OptionError as _error:
        sys.stderr.write('Invalid -W option ignored: ' + str(_error) + '\n')
