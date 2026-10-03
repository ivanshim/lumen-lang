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


def ismethod(object):
    # A bound method keeps the function it binds under __func__ and the
    # receiver it binds to under __self__.
    return hasattr(object, '__func__') and hasattr(object, '__self__')


def isfunction(object):
    # A plain function: a callable keeping a compiled body of its own,
    # neither a class nor a method.
    return (callable(object) and not isclass(object) and not ismethod(object)
            and getattr(object, '__code__', None) is not None)


_void = object()


def _signature_is_functionlike(obj):
    """Private helper to test if `obj` is a duck type of FunctionType.

    A good example of such objects are functions compiled with
    Cython, which have all attributes that a pure Python function
    would have, but have their code statically compiled.
    """

    if not callable(obj) or isclass(obj):
        # All function-like objects are obviously callables,
        # and not classes.
        return False

    name = getattr(obj, '__name__', None)
    code = getattr(obj, '__code__', None)
    defaults = getattr(obj, '__defaults__', _void)  # Important to use _void ...
    kwdefaults = getattr(obj, '__kwdefaults__', _void)  # ... and not None here

    return (isinstance(code, types.CodeType) and
            isinstance(name, str) and
            (defaults is None or isinstance(defaults, tuple)) and
            (kwdefaults is None or isinstance(kwdefaults, dict)))


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
    return bool(f.__code__.co_flags & flag)


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


def __getattr__(name):
    raise 'NotImplementedError: inspect.' + name + ' needs to read a compiled body, which this runtime does not hand out'


# A signature reconstructed from the code fields the runtime exposes.
# Frame walking and builtin text signatures remain unavailable.
class Signature:
    @classmethod
    def from_callable(cls, obj, *, follow_wrapped=True, globals=None, locals=None,
                      eval_str=False, annotation_format=1):
        if follow_wrapped:
            seen = set()
            while hasattr(obj, '__wrapped__'):
                if id(obj) in seen:
                    raise ValueError('wrapper loop when unwrapping ' + repr(obj))
                seen.add(id(obj))
                obj = obj.__wrapped__
        given = getattr(obj, '__signature__', None)
        if given is not None:
            if not isinstance(given, cls):
                raise TypeError('unexpected object in __signature__ attribute')
            return given
        bound = getattr(obj, '__self__', None) is not None
        if hasattr(obj, '__func__'):
            obj = obj.__func__
        if isinstance(obj, type):
            obj = obj.__init__
            bound = True
        elif not hasattr(obj, '__code__'):
            if not callable(obj):
                raise TypeError(repr(obj) + ' is not a callable object')
            obj = obj.__call__
            bound = True
        code = getattr(obj, '__code__', None)
        if code is None:
            raise ValueError('no signature found for builtin ' + repr(obj))
        names = code.co_varnames
        count = code.co_argcount
        posonly = code.co_posonlyargcount
        kwonly = code.co_kwonlyargcount
        defaults = getattr(obj, '__defaults__', None) or ()
        kwdefaults = getattr(obj, '__kwdefaults__', None) or {}
        parts = []
        for i in range(1 if bound else 0, count):
            part = names[i]
            if i >= count - len(defaults):
                part += '=' + repr(defaults[i - count + len(defaults)])
            parts.append(part)
            if i + 1 == posonly:
                parts.append('/')
        cursor = count + kwonly
        if code.co_flags & CO_VARARGS:
            parts.append('*' + names[cursor])
            cursor += 1
        elif kwonly:
            parts.append('*')
        for i in range(count, count + kwonly):
            part = names[i]
            if part in kwdefaults:
                part += '=' + repr(kwdefaults[part])
            parts.append(part)
        if code.co_flags & CO_VARKEYWORDS:
            parts.append('**' + names[cursor])
        result = cls()
        result._text = '(' + ', '.join(parts) + ')'
        return result

    def __str__(self):
        return self._text


def signature(obj, *, follow_wrapped=True, globals=None, locals=None,
              eval_str=False, annotation_format=1):
    return Signature.from_callable(obj, follow_wrapped=follow_wrapped,
                                   globals=globals, locals=locals,
                                   eval_str=eval_str,
                                   annotation_format=annotation_format)
