# Runtime entry points for CPython v3.14.8 Python/context.c (PSF License).
# Binding storage and token validation live in the Python-only native bridge.
from types import GenericAlias

class _Missing:
    def __repr__(self):
        return '<Token.MISSING>'

_missing = _Missing()
_unset = object()
_repr_running = set()

def _no_subclass(cls, **kwargs):
    raise TypeError("type '_contextvars." + cls.__bases__[0].__name__ + "' is not an acceptable base type")

class ContextVar:
    __module__ = '_contextvars'
    __slots__ = ('_handle', '_name', '_default')
    __init_subclass__ = classmethod(_no_subclass)

    def __init__(self, *args, **kwargs):
        if len(args) != 1:
            raise TypeError('ContextVar() takes exactly 1 positional argument (' + str(len(args)) + ' given)')
        if not isinstance(args[0], str):
            raise TypeError('context variable name must be a str')
        for key in kwargs:
            if key != 'default':
                raise TypeError("'" + key + "' is an invalid keyword argument for ContextVar()")
        hash(args[0])
        self._name = args[0]
        self._default = kwargs.get('default', _unset)
        self._handle = __context_native__('var', 0, self)

    @property
    def name(self):
        return self._name

    def get(self, *args):
        if len(args) > 1:
            raise TypeError('get expected at most 1 argument, got ' + str(len(args)))
        found, value = __context_native__('get', self._handle)
        if found:
            return value
        if args:
            return args[0]
        if self._default is not _unset:
            return self._default
        raise LookupError(self)

    def set(self, value, /):
        token = object.__new__(Token)
        token._handle = __context_native__('set', self._handle, value)
        return token

    def reset(self, token, /):
        if not isinstance(token, Token):
            raise TypeError('expected an instance of Token, got ' + repr(token))
        return __context_native__('reset', self._handle, token._handle, repr(token))

    def __repr__(self):
        serial = id(self)
        if serial in _repr_running:
            return '...'
        _repr_running.add(serial)
        try:
            shown = '<ContextVar name=' + repr(self._name)
            if self._default is not _unset:
                shown += ' default=' + repr(self._default)
            return shown + ' at ' + hex(serial) + '>'
        finally:
            _repr_running.remove(serial)

    @classmethod
    def __class_getitem__(cls, item):
        return GenericAlias(cls, item)

class Token:
    __module__ = '_contextvars'
    __slots__ = ('_handle',)
    __init_subclass__ = classmethod(_no_subclass)
    MISSING = _missing

    def __new__(cls, *args, **kwargs):
        raise RuntimeError('Tokens can only be created by ContextVars')

    @property
    def var(self):
        return __context_native__('token', self._handle)[0]

    @property
    def old_value(self):
        found, value = __context_native__('token', self._handle)[1]
        return value if found else self.MISSING

    def __repr__(self):
        var, old, used = __context_native__('token', self._handle)
        return '<Token' + (' used' if used else '') + ' var=' + repr(var) + ' at ' + hex(id(self)) + '>'

    def __enter__(self):
        return self

    def __exit__(self, exc_type, value, traceback):
        self.var.reset(self)

    @classmethod
    def __class_getitem__(cls, item):
        return GenericAlias(cls, item)

class Context:
    __module__ = '_contextvars'
    __slots__ = ('_handle',)
    __hash__ = None
    __init_subclass__ = classmethod(_no_subclass)

    def __init__(self, *args, **kwargs):
        if args or kwargs:
            raise TypeError('Context() does not accept any arguments')
        self._handle = __context_native__('new')

    def copy(self):
        ctx = object.__new__(Context)
        ctx._handle = __context_native__('copy', self._handle)
        return ctx

    def run(self, /, *args, **kwargs):
        if not args:
            raise TypeError('run() missing 1 required positional argument')
        previous = __context_native__('enter', self._handle, repr(self))
        try:
            return args[0](*args[1:], **kwargs)
        finally:
            __context_native__('leave', previous)

    def _lookup(self, key):
        if not isinstance(key, ContextVar):
            raise TypeError('a ContextVar key was expected, got ' + repr(key))
        return __context_native__('lookup', self._handle, key._handle)

    def __getitem__(self, key):
        found, value = self._lookup(key)
        if found:
            return value
        raise KeyError(key)

    def __contains__(self, key):
        return self._lookup(key)[0]

    def get(self, key, default=None, /):
        found, value = self._lookup(key)
        return value if found else default

    def items(self):
        return iter(__context_native__('entries', self._handle))

    def keys(self):
        entries = __context_native__('entries', self._handle)
        return iter([entry[0] for entry in entries])

    def values(self):
        entries = __context_native__('entries', self._handle)
        return iter([entry[1] for entry in entries])

    def __iter__(self):
        return self.keys()

    def __len__(self):
        return len(__context_native__('entries', self._handle))

    def __eq__(self, other):
        if not isinstance(other, Context):
            return NotImplemented
        left = __context_native__('entries', self._handle)
        right = __context_native__('entries', other._handle)
        return dict(left) == dict(right)

    def __ne__(self, other):
        value = self.__eq__(other)
        return value if value is NotImplemented else not value

    def __repr__(self):
        return '<_contextvars.Context object at ' + hex(id(self)) + '>'


def copy_context():
    ctx = object.__new__(Context)
    ctx._handle = __context_native__('copy', __context_native__('current'))
    return ctx
