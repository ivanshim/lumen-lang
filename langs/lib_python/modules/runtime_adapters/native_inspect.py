# What a program can find out about the things it is made of.
#
# Functions expose code objects and generators expose their suspended
# frames and running state. The interpreter does not keep a Python-level
# list of all active calls, so stack() and currentframe() remain unavailable.
# The helpers below use the available code and frame details.

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
            if getattr(obj, '__text_signature__', None) == '()':
                return cls()
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
