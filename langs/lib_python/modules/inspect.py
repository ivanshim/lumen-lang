# What a program can find out about the things it is made of.
#
# Functions expose code objects and generators expose their suspended
# frames and running state. The interpreter does not keep a Python-level
# list of all active calls, so stack() and currentframe() remain unavailable.
# The helpers below use the available code and frame details.

import functools
import types

# The flags CPython sets on a compiled body. Code objects expose co_flags
# so a program can compare these values with a function's compiled flags.
CO_OPTIMIZED = 1
CO_NEWLOCALS = 2
CO_VARARGS = 4
CO_VARKEYWORDS = 8
CO_NESTED = 16
CO_GENERATOR = 32
CO_NOFREE = 64
CO_COROUTINE = 128
CO_ITERABLE_COROUTINE = 256
CO_ASYNC_GENERATOR = 512

# The four words a generator's state is named by, and the same four for
# a coroutine.
GEN_CREATED = 'GEN_CREATED'
GEN_RUNNING = 'GEN_RUNNING'
GEN_SUSPENDED = 'GEN_SUSPENDED'
GEN_CLOSED = 'GEN_CLOSED'

CORO_CREATED = 'CORO_CREATED'
CORO_RUNNING = 'CORO_RUNNING'
CORO_SUSPENDED = 'CORO_SUSPENDED'
CORO_CLOSED = 'CORO_CLOSED'


def isclass(object):
    # A class is the one kind of thing this runtime can tell apart from
    # the rest without reading any insides.
    return isinstance(object, type)


def getmro(cls):
    return cls.__mro__


def cleandoc(doc):
    # Take the indentation a docstring picked up from the block it was
    # written in back off, the way CPython does: the first line loses
    # its leading space, and every line after loses the smallest
    # indentation any of them has.
    lines = doc.expandtabs().split('\n')
    margin = -1
    for line in lines[1:]:
        stripped = line.lstrip()
        if not stripped:
            continue
        indent = len(line) - len(stripped)
        if margin < 0 or indent < margin:
            margin = indent
    cleaned = [lines[0].strip()]
    if margin > 0:
        for line in lines[1:]:
            cleaned.append(line[margin:].rstrip())
    else:
        for line in lines[1:]:
            cleaned.append(line.rstrip())
    while cleaned and not cleaned[-1]:
        cleaned.pop()
    while cleaned and not cleaned[0]:
        cleaned.pop(0)
    return '\n'.join(cleaned)


def getdoc(object):
    if not hasattr(object, '__doc__'):
        return None
    doc = object.__doc__
    if doc is None:
        return None
    return cleandoc(doc)


def stack(context=1):
    # CPython walks the calls in progress and hands back a record for
    # each, with the frame, the file and the line. No kernel here keeps
    # the calls in progress as anything a program can reach, so an empty
    # list would read as a program called from nowhere, which is never
    # true. It refuses instead.
    raise 'NotImplementedError: inspect.stack needs the calls in progress as objects, which this runtime does not keep'


def currentframe():
    raise 'NotImplementedError: inspect.currentframe needs a frame object, which this runtime does not keep'


def getgeneratorstate(generator):
    if generator.gi_running:
        return GEN_RUNNING
    if generator.gi_suspended:
        return GEN_SUSPENDED
    if generator.gi_frame is None:
        return GEN_CLOSED
    return GEN_CREATED


def getcoroutinestate(coroutine):
    if coroutine.cr_running:
        return CORO_RUNNING
    if coroutine.cr_suspended:
        return CORO_SUSPENDED
    if coroutine.cr_frame is None:
        return CORO_CLOSED
    return CORO_CREATED


# --------------------------------------------------------------------
# Class and function questions, after CPython's Lib/inspect.py at the
# suite's commit 3b564385e4c9 under the PSF licence, simplified where the
# reference reads innards this runtime does not keep: a class's flags
# word, and the static mro read through type.__dict__.

_sentinel = object()


def _probe_function():
    pass


class _ProbeClass:
    def _probe_method(self):
        pass


_FunctionType = type(_probe_function)
_MethodType = type(_ProbeClass()._probe_method)


def isfunction(object):
    """Return true if the object is a user-defined function."""
    return isinstance(object, _FunctionType)


def ismethod(object):
    """Return true if the object is an instance method."""
    return isinstance(object, _MethodType)


def _signature_is_functionlike(obj):
    # A duck-typed function, the way CPython's helper of this name tells
    # one: something that is not a class yet carries a code object with
    # the four bookkeeping members a function has.
    if isinstance(obj, type):
        return False
    code = getattr(obj, '__code__', None)
    if code is None or not hasattr(code, 'co_flags'):
        return False
    for name in ('__defaults__', '__kwdefaults__', '__annotations__'):
        if not hasattr(obj, name):
            return False
    return True


def _has_code_flag(f, flag):
    """Return true if ``f`` is a function (or a method or functools.partial
    wrapper wrapping a function or a functools.partialmethod wrapping a
    function) whose code object has the given ``flag``
    set in its flags."""
    f = functools._unwrap_partialmethod(f)
    while ismethod(f):
        f = f.__func__
    f = functools._unwrap_partial(f)
    if not (isfunction(f) or _signature_is_functionlike(f)):
        return False
    # If it's a pure Python function, or an object that is duck type
    # of a Python function (Cython and Mock functions, for instance), then:
    return bool(f.__code__.co_flags & flag)


def isgeneratorfunction(obj):
    """Return true if the object is a user-defined generator function."""
    return _has_code_flag(obj, CO_GENERATOR)


# A marker for markcoroutinefunction and iscoroutinefunction.
_is_coroutine_mark = object()


def _has_coroutine_mark(f):
    while ismethod(f):
        f = f.__func__
    f = functools._unwrap_partial(f)
    return getattr(f, "_is_coroutine_marker", None) is _is_coroutine_mark


def markcoroutinefunction(func):
    """
    Decorator to ensure callable is recognised as a coroutine function.
    """
    if hasattr(func, '__func__'):
        func = func.__func__
    func._is_coroutine_marker = _is_coroutine_mark
    return func


def iscoroutinefunction(obj):
    """Return true if the object is a coroutine function.

    Coroutine functions are normally defined with "async def" syntax, but may
    be marked via markcoroutinefunction.
    """
    return _has_code_flag(obj, CO_COROUTINE) or _has_coroutine_mark(obj)


def isasyncgenfunction(obj):
    """Return true if the object is an asynchronous generator function.

    Asynchronous generator functions are defined with "async def"
    syntax and have "yield" expressions in their body.
    """
    return _has_code_flag(obj, CO_ASYNC_GENERATOR)


def isabstract(object):
    """Return true if the object is an abstract base class (ABC). The
    reference reads a flags word a class here does not keep; the set of
    abstract method names answers the same question."""
    if not isinstance(object, type):
        return False
    return bool(getattr(object, '__abstractmethods__', False))


def _check_instance(obj, attr):
    try:
        instance_dict = object.__getattribute__(obj, "__dict__")
    except AttributeError:
        instance_dict = {}
    return instance_dict.get(attr, _sentinel)


def _shadowed_dict(klass):
    # The __dict__ member a class's own namespace names, or nothing:
    # CPython reads it through a line of weak references; the class's
    # own namespace answers the same question directly here.
    try:
        return klass.__dict__.get("__dict__", _sentinel)
    except (AttributeError, TypeError):
        return _sentinel


def _check_class(klass, attr):
    for entry in getattr(klass, '__mro__', (klass,)):
        if _shadowed_dict(entry) is _sentinel:
            try:
                return entry.__dict__[attr]
            except (KeyError, AttributeError, TypeError):
                pass
    return _sentinel


def getattr_static(obj, attr, default=_sentinel):
    """Retrieve attributes without triggering dynamic lookup via the
       descriptor protocol,  __getattr__ or __getattribute__.

       Note: this function may not be able to retrieve all attributes
       that getattr can fetch (like dynamically created attributes)
       and may find attributes that getattr can't (like descriptors
       that raise AttributeError). It can also return descriptor objects
       instead of instance members in some cases. See the
       documentation for details.
    """
    instance_result = _sentinel

    # The reference tells a class from an instance by the metaclass's
    # own line; the metaclass root here answers no __mro__ of its own,
    # so the same question is asked of the kind directly.
    objtype = type(obj)
    if not isinstance(obj, type):
        klass = objtype
        dict_attr = _shadowed_dict(objtype)
        if (dict_attr is _sentinel or
            type(dict_attr) is types.MemberDescriptorType):
            instance_result = _check_instance(obj, attr)
    else:
        klass = obj

    klass_result = _check_class(klass, attr)

    if instance_result is not _sentinel and klass_result is not _sentinel:
        if _check_class(type(klass_result), "__get__") is not _sentinel and (
            _check_class(type(klass_result), "__set__") is not _sentinel
            or _check_class(type(klass_result), "__delete__") is not _sentinel
        ):
            return klass_result

    if instance_result is not _sentinel:
        return instance_result
    if klass_result is not _sentinel:
        return klass_result

    if obj is klass:
        # for types we check the metaclass too
        for entry in getattr(type(klass), '__mro__', ()):
            if (
                _shadowed_dict(type(entry)) is _sentinel
                and attr in entry.__dict__
            ):
                return entry.__dict__[attr]
    if default is not _sentinel:
        return default
    raise AttributeError(attr)


def _descriptor_get(descriptor, obj):
    if isclass(descriptor):
        return descriptor
    get = getattr(descriptor, '__get__', _sentinel)
    if get is _sentinel:
        return descriptor
    return get(obj, type(obj))

def __getattr__(name):
    raise 'NotImplementedError: inspect.' + name + ' needs to read a compiled body, which this runtime does not hand out'
