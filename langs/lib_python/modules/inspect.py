# What a program can find out about the things it is made of.
#
# Functions expose code objects and generators expose their suspended
# frames and running state. The interpreter keeps the calls in progress
# as frame objects that sys._getframe hands back, so stack() and
# currentframe() walk that chain here. The helpers below use the
# available code and frame details.

import functools
import sys
import types
from collections import namedtuple

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


# The five fields CPython gives a frame's place. The position of the
# code that raised it is a separate attribute, not a sixth field, so a
# record still unpacks into five values as CPython's does.
_Traceback = namedtuple('_Traceback', ['filename', 'lineno', 'function', 'code_context', 'index'])


class Traceback(_Traceback):
    def __new__(cls, filename, lineno, function, code_context, index, *, positions=None):
        instance = super().__new__(cls, filename, lineno, function, code_context, index)
        instance.positions = positions
        return instance

    def __len__(self):
        # A record is a tuple of its fields. The runtime does not reach
        # a builtin kind's own length through a subclass that holds a
        # frame, so the field count is named here.
        return len(_Traceback._fields)

    def __repr__(self):
        return ('Traceback(filename={!r}, lineno={!r}, function={!r}, '
                'code_context={!r}, index={!r}, positions={!r})'.format(
                self.filename, self.lineno, self.function, self.code_context,
                self.index, self.positions))


# The record of one call: the frame itself and the five fields of its
# place. The frame's position is the same separate attribute.
_FrameInfo = namedtuple('_FrameInfo', ['frame'] + list(Traceback._fields))


class FrameInfo(_FrameInfo):
    def __new__(cls, frame, filename, lineno, function, code_context, index, *, positions=None):
        instance = super().__new__(cls, frame, filename, lineno, function, code_context, index)
        instance.positions = positions
        return instance

    def __len__(self):
        # A record is a tuple of its fields. The runtime does not reach
        # a builtin kind's own length through a subclass that holds a
        # frame, so the field count is named here.
        return len(_FrameInfo._fields)

    def __repr__(self):
        return ('FrameInfo(frame={!r}, filename={!r}, lineno={!r}, function={!r}, '
                'code_context={!r}, index={!r}, positions={!r})'.format(
                self.frame, self.filename, self.lineno, self.function,
                self.code_context, self.index, self.positions))


def isframe(object):
    # A frame is the interpreter's own frame kind.
    return isinstance(object, types.FrameType)


def getframeinfo(frame, context=1):
    # The file, line and function name a frame stands at. The runtime
    # keeps no source text, so there are no lines of context to stand
    # beside them, as CPython has none when it cannot read the source.
    if not isframe(frame):
        raise TypeError('{!r} is not a frame or traceback object'.format(frame))
    code = frame.f_code
    return Traceback(code.co_filename, frame.f_lineno, code.co_name, None, None)


def getouterframes(frame, context=1):
    # A frame and every frame it was called from, innermost first.
    frames = []
    while frame is not None:
        info = getframeinfo(frame, context)
        frames.append(FrameInfo(frame, info.filename, info.lineno, info.function, info.code_context, info.index, positions=info.positions))
        frame = frame.f_back
    return frames


def stack(context=1):
    # The frames of the calls in progress, the caller of stack first.
    return getouterframes(sys._getframe(1), context)


def currentframe():
    # The frame of the caller of currentframe.
    return sys._getframe(1)


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
# v3.14.8 release under the PSF licence, simplified where the
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






class _void:
    """A private marker - used in Parameter & Signature."""










def isgeneratorfunction(obj):
    """Return true if the object is a user-defined generator function."""
    return _has_code_flag(obj, CO_GENERATOR)


# A marker for markcoroutinefunction and iscoroutinefunction.
_is_coroutine_mark = object()








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


_static_getmro = type.__dict__['__mro__'].__get__
_get_dunder_dict_of_class = type.__dict__['__dict__'].__get__


def _check_instance(obj, attr):
    instance_dict = {}
    try:
        instance_dict = object.__getattribute__(obj, "__dict__")
    except AttributeError:
        pass
    return dict.get(instance_dict, attr, _sentinel)


def _check_class(klass, attr):
    for entry in _static_getmro(klass):
        if _shadowed_dict(type(entry)) is _sentinel:
            namespace = _get_dunder_dict_of_class(entry)
            if attr in namespace:
                return namespace[attr]
    return _sentinel


def _shadowed_dict(klass):
    # The __dict__ a class's own namespace names: CPython caches the
    # answer through a line of weak references; each namespace is asked
    # through the kind's own descriptor here instead, so a metaclass
    # answering for those names is never consulted.
    for entry in _static_getmro(klass):
        dunder_dict = _get_dunder_dict_of_class(entry)
        if '__dict__' in dunder_dict:
            class_dict = dunder_dict['__dict__']
            if not (type(class_dict) is types.GetSetDescriptorType and
                    class_dict.__name__ == "__dict__" and
                    class_dict.__objclass__ is entry):
                return class_dict
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

    objtype = type(obj)
    if type not in _static_getmro(objtype):
        klass = objtype
        dict_attr = _shadowed_dict(klass)
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
        for entry in _static_getmro(type(klass)):
            if _shadowed_dict(type(entry)) is _sentinel:
                namespace = _get_dunder_dict_of_class(entry)
                if attr in namespace:
                    return namespace[attr]
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
# Coroutine recognition follows the flags and marker rules in CPython 3.14.8.
_is_coroutine_mark = object()



def __getattr__(name):
    if name.startswith('__'):
        raise AttributeError("module 'inspect' has no attribute '" + name + "'")
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


# Coroutine recognition follows CPython v3.14.8 inspect; PSF License.
import functools
import types
_void = object()
_is_coroutine_mark = object()

def ismethod(object):
    """Return true if the object is an instance method."""
    return isinstance(object, types.MethodType)

def isfunction(object):
    """Return true if the object is a user-defined function.

    Function objects provide these attributes:
        __doc__         documentation string
        __name__        name with which this function was defined
        __qualname__    qualified name of this function
        __module__      name of the module the function was defined in or None
        __code__        code object containing compiled function bytecode
        __defaults__    tuple of any default values for arguments
        __globals__     global namespace in which this function was defined
        __annotations__ dict of parameter annotations
        __kwdefaults__  dict of keyword only parameters with defaults
        __dict__        namespace which is supporting arbitrary function attributes
        __closure__     a tuple of cells or None
        __type_params__ tuple of type parameters"""
    return isinstance(object, types.FunctionType)

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
    defaults = getattr(obj, '__defaults__', _void) # Important to use _void ...
    kwdefaults = getattr(obj, '__kwdefaults__', _void) # ... and not None here

    return (isinstance(code, types.CodeType) and
            isinstance(name, str) and
            (defaults is None or isinstance(defaults, tuple)) and
            (kwdefaults is None or isinstance(kwdefaults, dict)))

def _has_code_flag(f, flag):
    """Return true if ``f`` is a function (or a method or functools.partial
    wrapper wrapping a function or a functools.partialmethod wrapping a
    function) whose code object has the given ``flag``
    set in its flags."""
    f = _unwrap_coroutine_callable(f)
    while ismethod(f):
        f = f.__func__
    f = functools._unwrap_partial(f)
    if not (isfunction(f) or _signature_is_functionlike(f)):
        return False
    # If it's a pure Python function, or an object that is duck type
    # of a Python function (Cython and Mock functions, for instance), then:
    return bool(f.__code__.co_flags & flag)

def _has_coroutine_mark(f):
    f = _unwrap_coroutine_callable(f)
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


def _unwrap_coroutine_callable(func):
    # Stop when no wrapper was removed. Native bound-method reads may create
    # equivalent handles, so their identity is not a termination condition.
    while True:
        method = getattr(func, '__partialmethod__', None)
        if isinstance(method, functools.partialmethod):
            func = method.func
        elif isinstance(func, (functools.partialmethod, functools.partial)):
            func = func.func
        else:
            return func
