# Filters belong to the module; each entered context keeps its former
# list, and the records are shared with the caller who asked for them.
filters = []

class _State:
    def __init__(self):
        self.context = None
        self.seen = []

_state = _State()

class WarningMessage:
    def __init__(self, message, category, filename, lineno, file=None, line=None, source=None):
        self.message = message
        self.category = category
        self.filename = filename
        self.lineno = lineno
        self.file = file
        self.line = line
        self.source = source


def _fold(text):
    result = ''
    for letter in text:
        number = ord(letter)
        if number >= 65 and number <= 90:
            letter = chr(number + 32)
        if number > 127:
            raise NotImplementedError('warning filter case folding outside ASCII cannot run yet')
        result += letter
    return result


def _matches(pattern, text, folded=False):
    if pattern == '':
        return True
    if folded:
        pattern = _fold(pattern)
        text = _fold(text)
    if pattern[-2:] == '\\z' or pattern[-2:] == '\\Z':
        pattern = pattern[:-2] + '$'
    if pattern[:1] == '^':
        pattern = pattern[1:]
    for letter in pattern:
        if letter in '[](){}|':
            raise NotImplementedError('grouped warning filter expressions cannot run yet')
    from unittest import _match_at
    return _match_at(pattern, text)


def _check(action, category, lineno):
    if action not in ['error', 'ignore', 'always', 'default', 'once', 'module']:
        raise AssertionError('invalid action: ' + repr(action))
    if not isinstance(category(), Warning):
        raise AssertionError('category must be a Warning subclass')
    if lineno < 0:
        raise AssertionError('lineno must be an int >= 0')


def filterwarnings(action, message='', category=Warning, module='', lineno=0, append=False):
    global filters
    _check(action, category, lineno)
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
    filterwarnings(action, '', category, '', lineno, append)


def resetwarnings():
    global filters
    filters = []
    _state.seen = []


def _location(stacklevel):
    calls = __warning_calls()
    if stacklevel < 1:
        stacklevel = 1
    if stacklevel >= len(calls):
        raise NotImplementedError('warning stack level lies outside the known calls')
    frame = calls[stacklevel]
    return [frame['file'], frame['line']]


def warn(message, category=None, stacklevel=1, source=None, *, skip_file_prefixes=()):
    if len(skip_file_prefixes) != 0:
        raise NotImplementedError('warning frame prefix selection cannot run yet')
    if isinstance(message, Warning):
        category = type(message)
    elif category is None:
        category = UserWarning
    if not isinstance(category(), Warning):
        raise TypeError('category must be a Warning subclass')
    if not isinstance(message, Warning):
        message = category(message)
    place = _location(stacklevel)
    filename = place[0]
    module = filename
    for position in range(len(filename)):
        if filename[position] == '/':
            module = filename[position + 1:]
    if module[-3:] == '.py':
        module = module[:-3]
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
        key = [str(message), category]
        if action != 'once':
            key = [*key, module]
        if action == 'default':
            key = [*key, lineno]
        if key in _state.seen:
            return None
        _state.seen = [*_state.seen, key]
    context = _state.context
    if context is not None and context.record:
        __shared_append(context.records, WarningMessage(message, category, filename, lineno, source=source))
    else:
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
        self.records = __shared_list()

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
