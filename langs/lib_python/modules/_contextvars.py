# Context variables for the interpreter's single-threaded Python runtime.
# The interface follows CPython's contextvars; contexts own their values,
# and tokens restore a previous binding only in the context that made them.

_missing = object()

class Token:
    MISSING = _missing
    def __init__(self, var, old_value, context):
        self.var = var
        self.old_value = old_value
        self._context = context
        self._used = False

class ContextVar:
    def __init__(self, name, *, default=_missing):
        if not isinstance(name, str):
            raise TypeError('context variable name must be a str')
        self.name = name
        self._default = default

    def get(self, default=_missing):
        if self in _current._values:
            return _current._values[self]
        if default is not _missing:
            return default
        if self._default is not _missing:
            return self._default
        raise LookupError(self.name)

    def set(self, value):
        old = _current._values.get(self, _missing)
        token = Token(self, old, _current)
        _current._values[self] = value
        return token

    def reset(self, token):
        if not isinstance(token, Token):
            raise TypeError('expected an instance of Token')
        if token._used:
            raise RuntimeError('Token has already been used once')
        if token.var is not self:
            raise ValueError('Token was created by a different ContextVar')
        if token._context is not _current:
            raise ValueError('Token was created in a different Context')
        if token.old_value is _missing:
            del _current._values[self]
        else:
            _current._values[self] = token.old_value
        token._used = True

class Context:
    def __init__(self):
        self._values = {}
        self._entered = False

    def copy(self):
        result = Context()
        result._values = self._values.copy()
        return result

    def run(self, callable, *args, **kwargs):
        global _current
        if self._entered:
            raise RuntimeError('cannot enter context: already entered')
        previous = _current
        self._entered = True
        _current = self
        try:
            return callable(*args, **kwargs)
        finally:
            _current = previous
            self._entered = False

    def __getitem__(self, key):
        return self._values[key]

    def __contains__(self, key):
        return key in self._values

    def __iter__(self):
        return iter(self._values)

    def __len__(self):
        return len(self._values)

    def keys(self):
        return self._values.keys()

    def values(self):
        return self._values.values()

    def items(self):
        return self._values.items()

    def get(self, key, default=None):
        return self._values.get(key, default)

_current = Context()

def copy_context():
    return _current.copy()
