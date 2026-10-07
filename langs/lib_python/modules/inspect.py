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
# Frame walking and builtin text signatures remain unavailable, so a
# callable whose body the runtime cannot hand out has no signature and
# says so, exactly where the reference says the same of a builtin it
# has no text signature for.
_empty = _void()


class Parameter:
    POSITIONAL_ONLY = 0
    POSITIONAL_OR_KEYWORD = 1
    VAR_POSITIONAL = 2
    KEYWORD_ONLY = 3
    VAR_KEYWORD = 4
    empty = _empty

    def __init__(self, name, kind, *, default=_empty, annotation=_empty):
        self.name = name
        self.kind = kind
        self.default = default
        self.annotation = annotation

    def __str__(self):
        lead = {Parameter.VAR_POSITIONAL: '*', Parameter.VAR_KEYWORD: '**'}.get(self.kind, '')
        text = lead + self.name
        if self.default is not _empty:
            text += '=' + repr(self.default)
        return text

    def __repr__(self):
        return '<Parameter "%s">' % (self.name,)


def _render_parameters(parameters):
    # The separators CPython writes between parameters are decided by
    # their kinds, not carried alongside the text: a run of positional
    # only parameters closes with '/', and keyword only parameters that
    # no '*name' already introduced open with a bare '*'.
    result = []
    positional_only_pending = False
    keyword_only_pending = True
    for parameter in parameters:
        kind = parameter.kind
        if kind == Parameter.POSITIONAL_ONLY:
            positional_only_pending = True
        elif positional_only_pending:
            result.append('/')
            positional_only_pending = False
        if kind == Parameter.VAR_POSITIONAL:
            keyword_only_pending = False
        elif kind == Parameter.KEYWORD_ONLY and keyword_only_pending:
            result.append('*')
            keyword_only_pending = False
        result.append(str(parameter))
    if positional_only_pending:
        result.append('/')
    return '(' + ', '.join(result) + ')'


class Signature:
    def __init__(self, parameters=None):
        self._parameters = {}
        ordered = []
        if parameters is None:
            parameters = ()
        for parameter in parameters:
            self._parameters[parameter.name] = parameter
            ordered.append(parameter)
        self._text = _render_parameters(ordered)

    @property
    def parameters(self):
        return self._parameters

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
        self_object = getattr(obj, '__self__', None)
        bound = self_object is not None
        if hasattr(obj, '__func__'):
            obj = obj.__func__
        if isinstance(obj, type):
            original = obj
            obj = getattr(obj, '__init__', None)
            if obj is None or not hasattr(obj, '__code__'):
                raise ValueError('no signature found for builtin type ' + repr(original))
            bound = True
        elif not hasattr(obj, '__code__'):
            if not callable(obj):
                raise TypeError(repr(obj) + ' is not a callable object')
            # A builtin method bound to a class answers with the class
            # alone, so the class method takes no parameter the reference
            # would show; that is the shape of type.mro and the like.
            if isinstance(self_object, type):
                return cls()
            # A builtin without a body the runtime keeps has no signature,
            # the same verdict the reference reaches for one it cannot read.
            call = getattr(obj, '__call__', None)
            if call is None or not hasattr(call, '__code__'):
                raise ValueError('no signature found for builtin ' + repr(obj))
            obj = call
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
        parameters = []
        for i in range(1 if bound else 0, count):
            default = _empty
            if i >= count - len(defaults):
                default = defaults[i - count + len(defaults)]
            kind = Parameter.POSITIONAL_ONLY if i + 1 <= posonly else Parameter.POSITIONAL_OR_KEYWORD
            parameters.append(Parameter(names[i], kind, default=default))
        cursor = count + kwonly
        if code.co_flags & CO_VARARGS:
            parameters.append(Parameter(names[cursor], Parameter.VAR_POSITIONAL))
            cursor += 1
        for i in range(count, count + kwonly):
            default = kwdefaults[names[i]] if names[i] in kwdefaults else _empty
            parameters.append(Parameter(names[i], Parameter.KEYWORD_ONLY, default=default))
        if code.co_flags & CO_VARKEYWORDS:
            parameters.append(Parameter(names[cursor], Parameter.VAR_KEYWORD))
        return cls(parameters)

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
