# Filters and captured records share one holder, so imported callers
# see each other's changes. Leaving a capture restores its outer state.
class _State:
    def __init__(self):
        self.context = None
        self.filters = []

_state = _State()

class WarningMessage:
    def __init__(self, message, category, filename, lineno):
        self.message = message
        self.category = category
        self.filename = filename
        self.lineno = lineno

class UserWarning:
    def __init__(self, message):
        self.message = str(message)
        self.args = (message,)

def warn(message, category=None, stacklevel=1, source=None):
    if category is None:
        category = UserWarning
    if not isinstance(message, category):
        message = category(message)
    action = 'default'
    for rule in _state.filters:
        if rule[1] is None or isinstance(message, rule[1]):
            action = rule[0]
            break
    if action == 'ignore':
        return None
    if action == 'error':
        raise message
    if action != 'always' and action != 'default':
        raise 'NotImplementedError: this warning filter action is not supported'
    context = _state.context
    if context is not None and context.record:
        if stacklevel != 1 or source is not None:
            raise 'NotImplementedError: warning source attribution is not supported'
        record = WarningMessage(message, category, None, None)
        context.records = [*context.records, record]
        return None
    raise 'NotImplementedError: uncaptured warning display is not supported'

def simplefilter(action, category=None, lineno=0, append=False):
    if lineno != 0:
        raise 'NotImplementedError: warning filters by line are not supported'
    if append:
        _state.filters = [*_state.filters, [action, category]]
    else:
        _state.filters = [[action, category], *_state.filters]

def resetwarnings():
    _state.filters = []

class catch_warnings:
    def __init__(self, record=False, module=None, action=None, category=None, lineno=0, append=False, _internal=False):
        if module is not None:
            raise 'NotImplementedError: alternate warning modules are not supported'
        self.record = record
        self.internal = _internal
        self.action = action
        self.category = category
        self.lineno = lineno
        self.append = append
        self.entered = False
        self.records = []

    def __enter__(self):
        if self.entered:
            raise 'RuntimeError: warning capture is already entered'
        if self.record and not self.internal:
            raise 'NotImplementedError: shared warning record lists are not supported'
        self.entered = True
        self.previous = _state.context
        self.filters = _state.filters
        _state.context = self
        if self.action is not None:
            simplefilter(self.action, self.category, self.lineno, self.append)
        if self.record:
            return self.records
        return None

    def __exit__(self, kind, value, traceback):
        if not self.entered:
            raise 'RuntimeError: warning capture was not entered'
        _state.context = self.previous
        _state.filters = self.filters
        self.entered = False
        return False
