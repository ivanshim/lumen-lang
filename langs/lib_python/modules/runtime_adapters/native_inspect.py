# What a program can find out about the things it is made of.
#
# Functions expose code objects and generators expose their suspended
# frames and running state. Active frames come from sys._getframe; stack
# inspection follows their native f_back links.

import functools
import types
import annotationlib

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
    return getouterframes(sys._getframe(1), context)


def currentframe():
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
_static_missing = object()


def _static_mro(cls):
    return __static_namespace__(cls, 1)


def _static_shadowed_dict(cls):
    import types
    for base in _static_mro(cls):
        namespace = __static_namespace__(base, 0)
        if '__dict__' in namespace:
            return not isinstance(namespace['__dict__'], types.GetSetDescriptorType)
    return False


def _static_class_attribute(cls, name):
    for base in _static_mro(cls):
        if _static_shadowed_dict(type(base)):
            continue
        namespace = __static_namespace__(base, 0)
        if name in namespace:
            return namespace[name]
    return _static_missing


def _static_data_descriptor(value):
    if isinstance(value, property):
        return True
    cls = type(value)
    return (_static_class_attribute(cls, '__get__') is not _static_missing
            and (_static_class_attribute(cls, '__set__') is not _static_missing
                 or _static_class_attribute(cls, '__delete__') is not _static_missing))


def getattr_static(obj, attr, default=_static_missing):
    """Look in raw namespaces, preserving descriptors without calling them."""
    cls = type(obj)
    class_value = _static_class_attribute(cls, attr)
    if isinstance(obj, type):
        own = _static_class_attribute(obj, attr)
        if own is not _static_missing:
            return own
    else:
        own = _static_missing
        # A shadowed __dict__ may itself be a descriptor. Never invoke it.
        if not _static_shadowed_dict(cls):
            try:
                namespace = __static_namespace__(obj, 0)
            except (AttributeError, TypeError):
                namespace = None
            if isinstance(namespace, dict) and attr in namespace:
                own = namespace[attr]
        if class_value is not _static_missing and _static_data_descriptor(class_value):
            return class_value
        if own is not _static_missing:
            return own
    if class_value is not _static_missing:
        return class_value
    if default is not _static_missing:
        return default
    raise AttributeError(attr)


# Coroutine recognition follows the flags and marker rules in CPython 3.14.8.
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


# A signature reconstructed from the code fields the runtime exposes,
# with the parameter names, defaults and annotations a caller can read
# back one by one. Frame walking and builtin text signatures stay
# unavailable; an annotation's text is what the formatter below writes.
_empty = object()


def _strip_typing_prefix(text):
    # A typing alias is written in a signature without its typing
    # prefix, however deeply the aliases nest.
    return text.replace('typing.', '')


def formatannotation(annotation, base_module=None, *, quote_annotation_strings=True):
    if not quote_annotation_strings and isinstance(annotation, str):
        return annotation
    if getattr(annotation, '__module__', None) == 'typing':
        return _strip_typing_prefix(repr(annotation))
    if isinstance(annotation, types.GenericAlias):
        return str(annotation)
    if isinstance(annotation, type):
        if annotation.__module__ in ('builtins', base_module):
            return annotation.__qualname__
        return annotation.__module__ + '.' + annotation.__qualname__
    if isinstance(annotation, annotationlib.ForwardRef):
        return annotation.__forward_arg__
    return repr(annotation)


class _ParameterKind:
    # The five ways a value may be handed to a call, each named and
    # described as CPython names them, kept apart by identity.
    def __init__(self, value, name, description):
        self.value = value
        self.name = name
        self.description = description

    def __repr__(self):
        return self.name

    def __str__(self):
        return self.name


_POSITIONAL_ONLY = _ParameterKind(0, 'POSITIONAL_ONLY', 'positional-only')
_POSITIONAL_OR_KEYWORD = _ParameterKind(1, 'POSITIONAL_OR_KEYWORD', 'positional or keyword')
_VAR_POSITIONAL = _ParameterKind(2, 'VAR_POSITIONAL', 'variadic positional')
_KEYWORD_ONLY = _ParameterKind(3, 'KEYWORD_ONLY', 'keyword-only')
_VAR_KEYWORD = _ParameterKind(4, 'VAR_KEYWORD', 'variadic keyword')


class Parameter:
    # One named place in a call: its name, how it may be handed over,
    # the default it falls back to, and the annotation written for it.
    POSITIONAL_ONLY = _POSITIONAL_ONLY
    POSITIONAL_OR_KEYWORD = _POSITIONAL_OR_KEYWORD
    VAR_POSITIONAL = _VAR_POSITIONAL
    KEYWORD_ONLY = _KEYWORD_ONLY
    VAR_KEYWORD = _VAR_KEYWORD
    empty = _empty

    def __init__(self, name, kind, *, default=_empty, annotation=_empty):
        if not isinstance(name, str):
            raise TypeError('name must be a str, not ' + type(name).__name__)
        self._name = name
        self._kind = kind
        self._default = default
        self._annotation = annotation

    @property
    def name(self):
        return self._name

    @property
    def default(self):
        return self._default

    @property
    def annotation(self):
        return self._annotation

    @property
    def kind(self):
        return self._kind

    def replace(self, *, name=_empty, kind=_empty, annotation=_empty, default=_empty):
        if name is _empty:
            name = self._name
        if kind is _empty:
            kind = self._kind
        if annotation is _empty:
            annotation = self._annotation
        if default is _empty:
            default = self._default
        return type(self)(name, kind, default=default, annotation=annotation)

    def __str__(self):
        text = self._name
        if self._kind is _VAR_POSITIONAL:
            text = '*' + text
        elif self._kind is _VAR_KEYWORD:
            text = '**' + text
        if self._annotation is not _empty:
            text = text + ': ' + formatannotation(self._annotation)
        if self._default is not _empty:
            if self._annotation is not _empty:
                text = text + ' = ' + repr(self._default)
            else:
                text = text + '=' + repr(self._default)
        return text

    def __eq__(self, other):
        if self is other:
            return True
        if not isinstance(other, Parameter):
            return NotImplemented
        return (self._name == other._name and self._kind is other._kind
                and self._default == other._default
                and self._annotation == other._annotation)

    def __hash__(self):
        # The same fields equality reads: two parameters that compare
        # alike go into the same bucket, and a default or annotation
        # that cannot be an element of a set or a key of a map lets that
        # show through, as it does in the reference.
        return hash((self._name, self._kind, self._annotation, self._default))


class Signature:
    _parameter_cls = Parameter
    empty = _empty

    def __init__(self, parameters=None, *, return_annotation=_empty,
                 __validate_parameters__=True):
        if parameters is None:
            ordered = {}
        elif not __validate_parameters__:
            ordered = {param.name: param for param in parameters}
        else:
            # A row of parameters is checked as it is read: kinds only
            # move forward, a variadic kind stands at most once, a
            # default does not precede one that has none, and no name is
            # taken twice.
            ordered = {}
            top_kind = _POSITIONAL_ONLY
            seen_default = False
            seen_variadic = set()
            for param in parameters:
                kind = param.kind
                name = param.name
                if kind in (_VAR_POSITIONAL, _VAR_KEYWORD):
                    if kind in seen_variadic:
                        raise ValueError('more than one ' + kind.description + ' parameter')
                    seen_variadic.add(kind)
                if kind.value < top_kind.value:
                    raise ValueError('wrong parameter order: ' + top_kind.description
                                     + ' parameter before ' + kind.description + ' parameter')
                elif kind.value > top_kind.value:
                    top_kind = kind
                if kind in (_POSITIONAL_ONLY, _POSITIONAL_OR_KEYWORD):
                    if param.default is _empty:
                        if seen_default:
                            raise ValueError('non-default argument follows default argument')
                    else:
                        seen_default = True
                if name in ordered:
                    raise ValueError('duplicate parameter name: ' + repr(name))
                ordered[name] = param
        self._parameters = ordered
        self._return_annotation = return_annotation

    @property
    def parameters(self):
        return self._parameters

    @property
    def return_annotation(self):
        return self._return_annotation

    def replace(self, *, parameters=_empty, return_annotation=_empty):
        if parameters is _empty:
            parameters = self._parameters.values()
        if return_annotation is _empty:
            return_annotation = self._return_annotation
        return type(self)(parameters, return_annotation=return_annotation)

    def __str__(self):
        result = []
        render_pos_only_separator = False
        render_kw_only_separator = True
        for param in self._parameters.values():
            formatted = str(param)
            kind = param.kind
            if kind is _POSITIONAL_ONLY:
                render_pos_only_separator = True
            elif render_pos_only_separator:
                result.append('/')
                render_pos_only_separator = False
            if kind is _VAR_POSITIONAL:
                render_kw_only_separator = False
            elif kind is _KEYWORD_ONLY and render_kw_only_separator:
                result.append('*')
                render_kw_only_separator = False
            result.append(formatted)
        if render_pos_only_separator:
            result.append('/')
        rendered = '(' + ', '.join(result) + ')'
        if self._return_annotation is not _empty:
            rendered += ' -> ' + formatannotation(self._return_annotation)
        return rendered

    def format(self, max_width=None, quote_annotation_strings=True):
        # The multi-line renderer is not reproduced here; every caller
        # in this runtime reads the one-line form back.
        return str(self)

    def _hash_basis(self):
        # The positional and positional-or-keyword parameters keep their
        # order; keyword-only ones are read as a bag, since their order
        # is not part of a call's shape; the return annotation is read
        # as it stands.
        ordered = tuple(param for param in self._parameters.values()
                        if param.kind is not _KEYWORD_ONLY)
        kw_only = {param.name: param for param in self._parameters.values()
                   if param.kind is _KEYWORD_ONLY}
        return ordered, kw_only, self._return_annotation

    def __eq__(self, other):
        if self is other:
            return True
        if not isinstance(other, Signature):
            return NotImplemented
        return self._hash_basis() == other._hash_basis()

    def __hash__(self):
        ordered, kw_only, return_annotation = self._hash_basis()
        return hash((ordered, frozenset(kw_only.values()), return_annotation))

    @staticmethod
    def _metaclass_call(obj):
        # The __call__ a class's own kind wrote before the common kind,
        # which decides whether the class is called as a plain type or
        # through a hand-written metaclass. The method is handed over
        # bound to the kind that wrote it, so the class it is passed on
        # a call is not read as one of the caller's arguments.
        for entry in type(obj).__mro__:
            if entry is type:
                return None
            if '__call__' in entry.__dict__:
                method = entry.__dict__['__call__']
                bind = getattr(method, '__get__', None)
                if bind is None:
                    return method
                return bind(entry, type(entry))
        return None

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
            custom = cls._metaclass_call(obj)
            if custom is not None:
                if hasattr(custom, '__code__'):
                    return cls.from_callable(custom, follow_wrapped=follow_wrapped,
                                             globals=globals, locals=locals,
                                             eval_str=eval_str,
                                             annotation_format=annotation_format)
                raise ValueError('no signature found for builtin type ' + repr(obj))
            obj = getattr(obj, '__init__', None)
            bound = True
        elif not hasattr(obj, '__code__'):
            if not callable(obj):
                raise TypeError(repr(obj) + ' is not a callable object')
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
        try:
            annotations = annotationlib.get_annotations(obj, globals=globals, locals=locals,
                                                        eval_str=eval_str,
                                                        format=annotation_format)
        except Exception:
            annotations = {}
        params = []
        no_default = count - len(defaults)
        for i in range(1 if bound else 0, count):
            kind = _POSITIONAL_ONLY if i < posonly else _POSITIONAL_OR_KEYWORD
            if i >= no_default:
                params.append(Parameter(names[i], kind, default=defaults[i - no_default],
                                        annotation=annotations.get(names[i], _empty)))
            else:
                params.append(Parameter(names[i], kind,
                                        annotation=annotations.get(names[i], _empty)))
        cursor = count + kwonly
        if code.co_flags & CO_VARARGS:
            params.append(Parameter(names[cursor], _VAR_POSITIONAL,
                                    annotation=annotations.get(names[cursor], _empty)))
            cursor += 1
        for i in range(count, count + kwonly):
            if names[i] in kwdefaults:
                params.append(Parameter(names[i], _KEYWORD_ONLY, default=kwdefaults[names[i]],
                                        annotation=annotations.get(names[i], _empty)))
            else:
                params.append(Parameter(names[i], _KEYWORD_ONLY,
                                        annotation=annotations.get(names[i], _empty)))
        if code.co_flags & CO_VARKEYWORDS:
            params.append(Parameter(names[cursor], _VAR_KEYWORD,
                                    annotation=annotations.get(names[cursor], _empty)))
        result = cls()
        result._parameters = {param.name: param for param in params}
        result._return_annotation = annotations.get('return', _empty)
        return result


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


# Source inspection from CPython v3.14.8 Lib/inspect.py; PSF License.
import os
import sys
import types
import functools
import linecache
import importlib.machinery

modulesbyfile = {}
_filesbymodname = {}

def ismodule(object):
    """Return true if the object is a module."""
    return isinstance(object, types.ModuleType)


def ismethoddescriptor(object):
    """Return true if the object is a method descriptor.

    But not if ismethod(), isclass() or isfunction() is true.

    An object passing this test (for example, int.__add__) has a __get__
    attribute, but not a __set__ attribute or a __delete__ attribute.
    Beyond that, the set of attributes varies; __name__ is usually
    sensible, and __doc__ often is.

    Methods implemented via descriptors that also pass one of the other
    tests (ismethod(), isclass(), isfunction()) make this function return
    false, simply because those other tests promise more -- you can, for
    example, count on having the __func__ attribute when an object passes
    ismethod()."""
    if isclass(object) or ismethod(object) or isfunction(object):
        # mutual exclusion
        return False
    tp = type(object)
    return (hasattr(tp, "__get__")
            and not hasattr(tp, "__set__")
            and not hasattr(tp, "__delete__"))


def isdatadescriptor(object):
    """Return true if the object is a data descriptor.

    But not if ismethod(), isclass() or isfunction() is true.

    Data descriptors have a __set__ or a __delete__ attribute.  Examples are
    properties, getsets, and members.  For the latter two (defined only in C
    extension modules) more specific tests are available as well:
    isgetsetdescriptor() and ismemberdescriptor(), respectively.

    Typically, data descriptors will also have __name__ and __doc__ attributes
    (properties, getsets, and members have both of these attributes), but this
    is not guaranteed."""
    if isclass(object) or ismethod(object) or isfunction(object):
        # mutual exclusion
        return False
    tp = type(object)
    return hasattr(tp, "__set__") or hasattr(tp, "__delete__")


def istraceback(object):
    """Return true if the object is a traceback.

    Traceback objects provide these attributes:
        tb_frame        frame object at this level
        tb_lasti        index of last attempted instruction in bytecode
        tb_lineno       current line number in Python source code
        tb_next         next inner traceback object (called by this level)"""
    return isinstance(object, types.TracebackType)


def isframe(object):
    """Return true if the object is a frame object.

    Frame objects provide these attributes:
        f_back          next outer frame object (this frame's caller)
        f_builtins      built-in namespace seen by this frame
        f_code          code object being executed in this frame
        f_globals       global namespace seen by this frame
        f_lasti         index of last attempted instruction in bytecode
        f_lineno        current line number in Python source code
        f_locals        local namespace seen by this frame
        f_trace         tracing function for this frame, or None
        f_trace_lines   is a tracing event triggered for each source line?
        f_trace_opcodes are per-opcode events being requested?

        clear()          used to clear all references to local variables"""
    return isinstance(object, types.FrameType)


def iscode(object):
    """Return true if the object is a code object.

    Code objects provide these attributes:
        co_argcount         number of arguments (not including *, ** args
                            or keyword only arguments)
        co_code             string of raw compiled bytecode
        co_cellvars         tuple of names of cell variables
        co_consts           tuple of constants used in the bytecode
        co_filename         name of file in which this code object was created
        co_firstlineno      number of first line in Python source code
        co_flags            bitmap: 1=optimized | 2=newlocals | 4=*arg | 8=**arg
                            | 16=nested | 32=generator | 64=nofree | 128=coroutine
                            | 256=iterable_coroutine | 512=async_generator
                            | 0x4000000=has_docstring
        co_freevars         tuple of names of free variables
        co_posonlyargcount  number of positional only arguments
        co_kwonlyargcount   number of keyword only arguments (not including ** arg)
        co_lnotab           encoded mapping of line numbers to bytecode indices
        co_name             name with which this code object was defined
        co_names            tuple of names other than arguments and function locals
        co_nlocals          number of local variables
        co_stacksize        virtual machine stack space required
        co_varnames         tuple of names of arguments and local variables
        co_qualname         fully qualified function name

        co_lines()          returns an iterator that yields successive bytecode ranges
        co_positions()      returns an iterator of source code positions for each bytecode instruction
        replace()           returns a copy of the code object with a new values"""
    return isinstance(object, types.CodeType)


def isbuiltin(object):
    """Return true if the object is a built-in function or method.

    Built-in functions and methods provide these attributes:
        __doc__         documentation string
        __name__        original name of this function or method
        __self__        instance to which a method is bound, or None"""
    return isinstance(object, types.BuiltinFunctionType)


def ismethodwrapper(object):
    """Return true if the object is a method wrapper."""
    return isinstance(object, types.MethodWrapperType)


def isroutine(object):
    """Return true if the object is any kind of function or method."""
    return (isbuiltin(object)
            or isfunction(object)
            or ismethod(object)
            or ismethoddescriptor(object)
            or ismethodwrapper(object)
            or isinstance(object, functools._singledispatchmethod_get))


def getfile(object):
    """Work out which source or compiled file an object was defined in."""
    if ismodule(object):
        if getattr(object, '__file__', None):
            return object.__file__
        raise TypeError('{!r} is a built-in module'.format(object))
    if isclass(object):
        if hasattr(object, '__module__'):
            module = sys.modules.get(object.__module__)
            if getattr(module, '__file__', None):
                return module.__file__
            if object.__module__ == '__main__':
                raise OSError('source code not available')
        raise TypeError('{!r} is a built-in class'.format(object))
    if ismethod(object):
        object = object.__func__
    if isfunction(object):
        object = object.__code__
    if istraceback(object):
        object = object.tb_frame
    if isframe(object):
        object = object.f_code
    if iscode(object):
        return object.co_filename
    raise TypeError('module, class, method, function, traceback, frame, or '
                    'code object was expected, got {}'.format(
                    type(object).__name__))


def getsourcefile(object):
    """Return the filename that can be used to locate an object's source.
    Return None if no way can be identified to get the source.
    """
    filename = getfile(object)
    all_bytecode_suffixes = importlib.machinery.BYTECODE_SUFFIXES[:]
    if any(filename.endswith(s) for s in all_bytecode_suffixes):
        filename = (os.path.splitext(filename)[0] +
                    importlib.machinery.SOURCE_SUFFIXES[0])
    elif any(filename.endswith(s) for s in
                 importlib.machinery.EXTENSION_SUFFIXES):
        return None
    elif filename.endswith(".fwork"):
        # Apple mobile framework markers are another type of non-source file
        return None

    # return a filename found in the linecache even if it doesn't exist on disk
    if filename in linecache.cache:
        return filename
    if os.path.exists(filename):
        return filename
    # only return a non-existent filename if the module has a PEP 302 loader
    module = getmodule(object, filename)
    if getattr(module, '__loader__', None) is not None:
        return filename
    elif getattr(getattr(module, "__spec__", None), "loader", None) is not None:
        return filename


def getabsfile(object, _filename=None):
    """Return an absolute path to the source or compiled file for an object.

    The idea is for each object to have a unique origin, so this routine
    normalizes the result as much as possible."""
    if _filename is None:
        _filename = getsourcefile(object) or getfile(object)
    return os.path.normcase(os.path.abspath(_filename))


def getmodule(object, _filename=None):
    """Return the module an object was defined in, or None if not found."""
    if ismodule(object):
        return object
    if hasattr(object, '__module__'):
        return sys.modules.get(object.__module__)

    # Try the filename to modulename cache
    if _filename is not None and _filename in modulesbyfile:
        return sys.modules.get(modulesbyfile[_filename])
    # Try the cache again with the absolute file name
    try:
        file = getabsfile(object, _filename)
    except (TypeError, FileNotFoundError):
        return None
    if file in modulesbyfile:
        return sys.modules.get(modulesbyfile[file])
    # Update the filename to module name cache and check yet again
    # Copy sys.modules in order to cope with changes while iterating
    for modname, module in sys.modules.copy().items():
        if ismodule(module) and hasattr(module, '__file__'):
            f = module.__file__
            if f == _filesbymodname.get(modname, None):
                # Have already mapped this module, so skip it
                continue
            _filesbymodname[modname] = f
            f = getabsfile(module)
            # Always map to the name the module knows itself by
            modulesbyfile[f] = modulesbyfile[
                os.path.realpath(f)] = module.__name__
    if file in modulesbyfile:
        return sys.modules.get(modulesbyfile[file])
    # Check the main module
    main = sys.modules['__main__']
    if not hasattr(object, '__name__'):
        return None
    if hasattr(main, object.__name__):
        mainobject = getattr(main, object.__name__)
        if mainobject is object:
            return main
    # Check builtins
    builtin = sys.modules['builtins']
    if hasattr(builtin, object.__name__):
        builtinobject = getattr(builtin, object.__name__)
        if builtinobject is object:
            return builtin


import tokenize


def findsource(object):
    """Return the entire source file and starting line number for an object.

    The argument may be a module, class, method, function, traceback, frame,
    or code object.  The source code is returned as a list of all the lines
    in the file and the line number indexes a line in that list.  An OSError
    is raised if the source code cannot be retrieved."""

    file = getsourcefile(object)
    if file:
        # Invalidate cache if needed.
        linecache.checkcache(file)
    else:
        file = getfile(object)
        # Allow filenames in form of "<something>" to pass through.
        # `doctest` monkeypatches `linecache` module to enable
        # inspection, so let `linecache.getlines` to be called.
        if (not (file.startswith('<') and file.endswith('>'))) or file.endswith('.fwork'):
            raise OSError('source code not available')

    module = getmodule(object, file)
    if module:
        lines = linecache.getlines(file, module.__dict__)
        if not lines and file.startswith('<') and hasattr(object, "__code__"):
            lines = linecache._getlines_from_code(object.__code__)
    else:
        lines = linecache.getlines(file)
    if not lines:
        raise OSError('could not get source code')

    if ismodule(object):
        return lines, 0

    if isclass(object):
        try:
            lnum = vars(object)['__firstlineno__'] - 1
        except (TypeError, KeyError):
            raise OSError('source code not available')
        if lnum >= len(lines):
            raise OSError('lineno is out of bounds')
        return lines, lnum

    if ismethod(object):
        object = object.__func__
    if isfunction(object):
        object = object.__code__
    if istraceback(object):
        object = object.tb_frame
    if isframe(object):
        object = object.f_code
    if iscode(object):
        if not hasattr(object, 'co_firstlineno'):
            raise OSError('could not find function definition')
        lnum = object.co_firstlineno - 1
        if lnum >= len(lines):
            raise OSError('lineno is out of bounds')
        return lines, lnum
    raise OSError('could not find code object')



class EndOfBlock(Exception): pass



class BlockFinder:
    """Provide a tokeneater() method to detect the end of a code block."""
    def __init__(self):
        self.indent = 0
        self.singleline = False
        self.started = False
        self.passline = False
        self.indecorator = False
        self.last = 1
        self.body_col0 = None

    def tokeneater(self, type, token, srowcol, erowcol, line):
        if not self.started and not self.indecorator:
            if type in (tokenize.INDENT, tokenize.COMMENT, tokenize.NL):
                pass
            elif token == "async":
                pass
            # skip any decorators
            elif token == "@":
                self.indecorator = True
            else:
                # For "def" and "class" scan to the end of the block.
                # For "lambda" and generator expression scan to
                # the end of the logical line.
                self.singleline = token not in ("def", "class")
                self.started = True
            self.passline = True    # skip to the end of the line
        elif type == tokenize.NEWLINE:
            self.passline = False   # stop skipping when a NEWLINE is seen
            self.last = srowcol[0]
            if self.singleline:
                raise EndOfBlock
            # hitting a NEWLINE when in a decorator without args
            # ends the decorator
            if self.indecorator:
                self.indecorator = False
        elif self.passline:
            pass
        elif type == tokenize.INDENT:
            if self.body_col0 is None and self.started:
                self.body_col0 = erowcol[1]
            self.indent = self.indent + 1
            self.passline = True
        elif type == tokenize.DEDENT:
            self.indent = self.indent - 1
            # the end of matching indent/dedent pairs end a block
            # (note that this only works for "def"/"class" blocks,
            #  not e.g. for "if: else:" or "try: finally:" blocks)
            if self.indent <= 0:
                raise EndOfBlock
        elif type == tokenize.COMMENT:
            if self.body_col0 is not None and srowcol[1] >= self.body_col0:
                # Include comments if indented at least as much as the block
                self.last = srowcol[0]
        elif self.indent == 0 and type not in (tokenize.COMMENT, tokenize.NL):
            # any other token on the same indentation level end the previous
            # block as well, except the pseudo-tokens COMMENT and NL.
            raise EndOfBlock



def getblock(lines):
    """Extract the block of code at the top of the given list of lines."""
    blockfinder = BlockFinder()
    try:
        tokens = tokenize.generate_tokens(iter(lines).__next__)
        for _token in tokens:
            blockfinder.tokeneater(*_token)
    except (EndOfBlock, IndentationError):
        pass
    except SyntaxError as e:
        if "unmatched" not in e.msg:
            raise e from None
        _, *_token_info = _token
        try:
            blockfinder.tokeneater(tokenize.NEWLINE, *_token_info)
        except (EndOfBlock, IndentationError):
            pass
    return lines[:blockfinder.last]



def getsourcelines(object):
    """Return a list of source lines and starting line number for an object.

    The argument may be a module, class, method, function, traceback, frame,
    or code object.  The source code is returned as a list of the lines
    corresponding to the object and the line number indicates where in the
    original source file the first line of code was found.  An OSError is
    raised if the source code cannot be retrieved."""
    object = unwrap(object)
    lines, lnum = findsource(object)

    if istraceback(object):
        object = object.tb_frame

    # for module or frame that corresponds to module, return all source lines
    if (ismodule(object) or
        (isframe(object) and object.f_code.co_name == "<module>")):
        return lines, 0
    else:
        return getblock(lines[lnum:]), lnum + 1



def getsource(object):
    """Return the text of the source code for an object.

    The argument may be a module, class, method, function, traceback, frame,
    or code object.  The source code is returned as a single string.  An
    OSError is raised if the source code cannot be retrieved."""
    lines, lnum = getsourcelines(object)
    return ''.join(lines)
from collections import namedtuple as _frame_tuple

class Traceback(_frame_tuple('_Traceback', 'filename lineno function code_context index')):
    def __new__(cls, filename, lineno, function, code_context, index, *, positions=None):
        item = super().__new__(cls, filename, lineno, function, code_context, index)
        item.positions = positions
        return item


def getframeinfo(frame, context=1):
    import linecache
    if hasattr(frame, 'tb_frame'):
        lineno = frame.tb_lineno
        frame = frame.tb_frame
    else:
        lineno = frame.f_lineno
    filename = frame.f_code.co_filename
    lines = linecache.getlines(filename, frame.f_globals)
    if context > 0 and lines:
        start = max(0, min(lineno - 1 - context // 2, len(lines) - context))
        code_context = lines[start:start + context]
        index = lineno - 1 - start
    else:
        code_context = index = None
    return Traceback(filename, lineno, frame.f_code.co_name, code_context, index)


class FrameInfo(_frame_tuple('_FrameInfo', 'frame filename lineno function code_context index')):
    def __new__(cls, frame, filename, lineno, function, code_context, index, *, positions=None):
        value = super().__new__(cls, frame, filename, lineno, function, code_context, index)
        value.positions = positions
        return value

    def __len__(self):
        return 6

    def __repr__(self):
        return ('FrameInfo(frame={!r}, filename={!r}, lineno={!r}, function={!r}, '
                'code_context={!r}, index={!r}, positions={!r})'.format(
                self.frame, self.filename, self.lineno, self.function,
                self.code_context, self.index, self.positions))


def getouterframes(frame, context=1):
    result = []
    while frame is not None:
        info = getframeinfo(frame, context)
        result.append(FrameInfo(frame, info.filename, info.lineno, info.function,
                               info.code_context, info.index, positions=info.positions))
        frame = frame.f_back
    return result
