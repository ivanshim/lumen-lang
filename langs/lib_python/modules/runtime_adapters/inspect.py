# Source: CPython Lib/inspect.py unwrap at v3.14.8 / 8e6e75d9102e; PSF License.
import sys
import re

def unwrap(func, *, stop=None):
    """Get the object wrapped by *func*.

   Follows the chain of :attr:`__wrapped__` attributes returning the last
   object in the chain.

   *stop* is an optional callback accepting an object in the wrapper chain
   as its sole argument that allows the unwrapping to be terminated early if
   the callback returns a true value. If the callback never returns a true
   value, the last object in the chain is returned as usual. For example,
   :func:`signature` uses this to stop unwrapping if any object in the
   chain has a ``__signature__`` attribute defined.

   :exc:`ValueError` is raised if a cycle is encountered.

    """
    f = func  # remember the original func for error reporting
    # Memoise by id to tolerate non-hashable objects, but store objects to
    # ensure they aren't destroyed, which would allow their IDs to be reused.
    memo = {id(f): f}
    recursion_limit = sys.getrecursionlimit()
    while not isinstance(func, type) and hasattr(func, '__wrapped__'):
        if stop is not None and stop(func):
            break
        func = func.__wrapped__
        id_func = id(func)
        if (id_func in memo) or (len(memo) >= recursion_limit):
            raise ValueError('wrapper loop when unwrapping {!r}'.format(f))
        memo[id_func] = func
    return func


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


# Inspect awaitable protocols; CPython v3.14.8 Lib/inspect.py, PSF License.
import collections.abc

def isawaitable(object):
    """Return true if object can be passed to an ``await`` expression."""
    return (isinstance(object, types.CoroutineType) or
            isinstance(object, types.GeneratorType) and
                bool(object.gi_code.co_flags & CO_ITERABLE_COROUTINE) or
            isinstance(object, collections.abc.Awaitable))

# Parameter and binding protocols: CPython v3.14.8 Lib/inspect.py; PSF License.
import enum
import itertools
from collections import OrderedDict
from keyword import iskeyword
from annotationlib import Format, ForwardRef
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


class _void:
    """A private marker - used in Parameter & Signature."""


class _empty:
    """Marker object for Signature.empty and Parameter.empty."""


class _ParameterKind(enum.IntEnum):
    POSITIONAL_ONLY = 'positional-only'
    POSITIONAL_OR_KEYWORD = 'positional or keyword'
    VAR_POSITIONAL = 'variadic positional'
    KEYWORD_ONLY = 'keyword-only'
    VAR_KEYWORD = 'variadic keyword'

    def __new__(cls, description):
        value = len(cls.__members__)
        member = int.__new__(cls, value)
        member._value_ = value
        member.description = description
        return member

    def __str__(self):
        return self.name

_POSITIONAL_ONLY = _ParameterKind.POSITIONAL_ONLY
_POSITIONAL_OR_KEYWORD = _ParameterKind.POSITIONAL_OR_KEYWORD
_VAR_POSITIONAL = _ParameterKind.VAR_POSITIONAL
_KEYWORD_ONLY = _ParameterKind.KEYWORD_ONLY
_VAR_KEYWORD = _ParameterKind.VAR_KEYWORD


def formatannotation(annotation, base_module=None, *, quote_annotation_strings=True):
    if not quote_annotation_strings and isinstance(annotation, str):
        return annotation
    if getattr(annotation, '__module__', None) == 'typing':
        def repl(match):
            text = match.group()
            return text.removeprefix('typing.')
        return re.sub(r'[\w\.]+', repl, repr(annotation))
    if isinstance(annotation, types.GenericAlias):
        return str(annotation)
    if isinstance(annotation, type):
        if annotation.__module__ in ('builtins', base_module):
            return annotation.__qualname__
        return annotation.__module__+'.'+annotation.__qualname__
    if isinstance(annotation, ForwardRef):
        return annotation.__forward_arg__
    return repr(annotation)


class Parameter:
    """Represents a parameter in a function signature.

    Has the following public attributes:

    * name : str
        The name of the parameter as a string.
    * default : object
        The default value for the parameter if specified.  If the
        parameter has no default value, this attribute is set to
        `Parameter.empty`.
    * annotation
        The annotation for the parameter if specified.  If the
        parameter has no annotation, this attribute is set to
        `Parameter.empty`.
    * kind
        Describes how argument values are bound to the parameter.
        Possible values: `Parameter.POSITIONAL_ONLY`,
        `Parameter.POSITIONAL_OR_KEYWORD`, `Parameter.VAR_POSITIONAL`,
        `Parameter.KEYWORD_ONLY`, `Parameter.VAR_KEYWORD`.
        Every value has a `description` attribute describing meaning.
    """

    __slots__ = ('_name', '_kind', '_default', '_annotation')

    POSITIONAL_ONLY         = _POSITIONAL_ONLY
    POSITIONAL_OR_KEYWORD   = _POSITIONAL_OR_KEYWORD
    VAR_POSITIONAL          = _VAR_POSITIONAL
    KEYWORD_ONLY            = _KEYWORD_ONLY
    VAR_KEYWORD             = _VAR_KEYWORD

    empty = _empty

    def __init__(self, name, kind, *, default=_empty, annotation=_empty):
        try:
            self._kind = _ParameterKind(kind)
        except ValueError:
            raise ValueError(f'value {kind!r} is not a valid Parameter.kind')
        if default is not _empty:
            if self._kind in (_VAR_POSITIONAL, _VAR_KEYWORD):
                msg = '{} parameters cannot have default values'
                msg = msg.format(self._kind.description)
                raise ValueError(msg)
        self._default = default
        self._annotation = annotation

        if name is _empty:
            raise ValueError('name is a required attribute for Parameter')

        if not isinstance(name, str):
            msg = 'name must be a str, not a {}'.format(type(name).__name__)
            raise TypeError(msg)

        if name[0] == '.' and name[1:].isdigit():
            # These are implicit arguments generated by comprehensions. In
            # order to provide a friendlier interface to users, we recast
            # their name as "implicitN" and treat them as positional-only.
            # See issue 19611.
            if self._kind != _POSITIONAL_OR_KEYWORD:
                msg = (
                    'implicit arguments must be passed as '
                    'positional or keyword arguments, not {}'
                )
                msg = msg.format(self._kind.description)
                raise ValueError(msg)
            self._kind = _POSITIONAL_ONLY
            name = 'implicit{}'.format(name[1:])
        elif name == '.format':
            # gh-151665: Hidden parameter of compiler-generated annotation and type
            # alias/typevar evaluators. Show it as "format".
            name = 'format'

        # It's possible for C functions to have a positional-only parameter
        # where the name is a keyword, so for compatibility we'll allow it.
        is_keyword = iskeyword(name) and self._kind is not _POSITIONAL_ONLY
        if is_keyword or not name.isidentifier():
            raise ValueError('{!r} is not a valid parameter name'.format(name))

        self._name = name

    def __reduce__(self):
        return (type(self),
                (self._name, self._kind),
                {'_default': self._default,
                 '_annotation': self._annotation})

    def __setstate__(self, state):
        self._default = state['_default']
        self._annotation = state['_annotation']

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

    def replace(self, *, name=_void, kind=_void,
                annotation=_void, default=_void):
        """Creates a customized copy of the Parameter."""

        if name is _void:
            name = self._name

        if kind is _void:
            kind = self._kind

        if annotation is _void:
            annotation = self._annotation

        if default is _void:
            default = self._default

        return type(self)(name, kind, default=default, annotation=annotation)

    def __str__(self):
        return self._format()

    def _format(self, *, quote_annotation_strings=True):
        kind = self.kind
        formatted = self._name

        # Add annotation and default value
        if self._annotation is not _empty:
            annotation = formatannotation(self._annotation,
                                          quote_annotation_strings=quote_annotation_strings)
            formatted = '{}: {}'.format(formatted, annotation)

        if self._default is not _empty:
            if self._annotation is not _empty:
                formatted = '{} = {}'.format(formatted, repr(self._default))
            else:
                formatted = '{}={}'.format(formatted, repr(self._default))

        if kind == _VAR_POSITIONAL:
            formatted = '*' + formatted
        elif kind == _VAR_KEYWORD:
            formatted = '**' + formatted

        return formatted

    __replace__ = replace

    def __repr__(self):
        return '<{} "{}">'.format(self.__class__.__name__, self)

    def __hash__(self):
        return hash((self._name, self._kind, self._annotation, self._default))

    def __eq__(self, other):
        if self is other:
            return True
        if not isinstance(other, Parameter):
            return NotImplemented
        return (self._name == other._name and
                self._kind == other._kind and
                self._default == other._default and
                self._annotation == other._annotation)


class BoundArguments:
    """Result of `Signature.bind` call.  Holds the mapping of arguments
    to the function's parameters.

    Has the following public attributes:

    * arguments : dict
        An ordered mutable mapping of parameters' names to arguments' values.
        Does not contain arguments' default values.
    * signature : Signature
        The Signature object that created this instance.
    * args : tuple
        Tuple of positional arguments values.
    * kwargs : dict
        Dict of keyword arguments values.
    """

    __slots__ = ('arguments', '_signature', '__weakref__')

    def __init__(self, signature, arguments):
        self.arguments = arguments
        self._signature = signature

    @property
    def signature(self):
        return self._signature

    @property
    def args(self):
        args = []
        for param_name, param in self._signature.parameters.items():
            if param.kind in (_VAR_KEYWORD, _KEYWORD_ONLY):
                break

            try:
                arg = self.arguments[param_name]
            except KeyError:
                # We're done here. Other arguments
                # will be mapped in 'BoundArguments.kwargs'
                break
            else:
                if param.kind == _VAR_POSITIONAL:
                    # *args
                    args.extend(arg)
                else:
                    # plain argument
                    args.append(arg)

        return tuple(args)

    @property
    def kwargs(self):
        kwargs = {}
        kwargs_started = False
        for param_name, param in self._signature.parameters.items():
            if not kwargs_started:
                if param.kind in (_VAR_KEYWORD, _KEYWORD_ONLY):
                    kwargs_started = True
                else:
                    if param_name not in self.arguments:
                        kwargs_started = True
                        continue

            if not kwargs_started:
                continue

            try:
                arg = self.arguments[param_name]
            except KeyError:
                pass
            else:
                if param.kind == _VAR_KEYWORD:
                    # **kwargs
                    kwargs.update(arg)
                else:
                    # plain keyword argument
                    kwargs[param_name] = arg

        return kwargs

    def apply_defaults(self):
        """Set default values for missing arguments.

        For variable-positional arguments (*args) the default is an
        empty tuple.

        For variable-keyword arguments (**kwargs) the default is an
        empty dict.
        """
        arguments = self.arguments
        new_arguments = []
        for name, param in self._signature.parameters.items():
            try:
                new_arguments.append((name, arguments[name]))
            except KeyError:
                if param.default is not _empty:
                    val = param.default
                elif param.kind is _VAR_POSITIONAL:
                    val = ()
                elif param.kind is _VAR_KEYWORD:
                    val = {}
                else:
                    # This BoundArguments was likely produced by
                    # Signature.bind_partial().
                    continue
                new_arguments.append((name, val))
        self.arguments = dict(new_arguments)

    def __eq__(self, other):
        if self is other:
            return True
        if not isinstance(other, BoundArguments):
            return NotImplemented
        return (self.signature == other.signature and
                self.arguments == other.arguments)

    def __setstate__(self, state):
        self._signature = state['_signature']
        self.arguments = state['arguments']

    def __getstate__(self):
        return {'_signature': self._signature, 'arguments': self.arguments}

    def __repr__(self):
        args = []
        for arg, value in self.arguments.items():
            args.append('{}={!r}'.format(arg, value))
        return '<{} ({})>'.format(self.__class__.__name__, ', '.join(args))


class Signature:
    """A Signature object represents the overall signature of a function.
    It stores a Parameter object for each parameter accepted by the
    function, as well as information specific to the function itself.

    A Signature object has the following public attributes and methods:

    * parameters : OrderedDict
        An ordered mapping of parameters' names to the corresponding
        Parameter objects (keyword-only arguments are in the same order
        as listed in `code.co_varnames`).
    * return_annotation : object
        The annotation for the return type of the function if specified.
        If the function has no annotation for its return type, this
        attribute is set to `Signature.empty`.
    * bind(*args, **kwargs) -> BoundArguments
        Creates a mapping from positional and keyword arguments to
        parameters.
    * bind_partial(*args, **kwargs) -> BoundArguments
        Creates a partial mapping from positional and keyword arguments
        to parameters (simulating 'functools.partial' behavior.)
    """

    __slots__ = ('_return_annotation', '_parameters')

    _parameter_cls = Parameter
    _bound_arguments_cls = BoundArguments

    empty = _empty

    def __init__(self, parameters=None, *, return_annotation=_empty,
                 __validate_parameters__=True):
        """Constructs Signature from the given list of Parameter
        objects and 'return_annotation'.  All arguments are optional.
        """

        if parameters is None:
            params = OrderedDict()
        else:
            if __validate_parameters__:
                params = OrderedDict()
                top_kind = _POSITIONAL_ONLY
                seen_default = False
                seen_var_parameters = set()

                for param in parameters:
                    kind = param.kind
                    name = param.name

                    if kind in (_VAR_POSITIONAL, _VAR_KEYWORD):
                        if kind in seen_var_parameters:
                            msg = f'more than one {kind.description} parameter'
                            raise ValueError(msg)

                        seen_var_parameters.add(kind)

                    if kind < top_kind:
                        msg = (
                            'wrong parameter order: {} parameter before {} '
                            'parameter'
                        )
                        msg = msg.format(top_kind.description,
                                         kind.description)
                        raise ValueError(msg)
                    elif kind > top_kind:
                        top_kind = kind

                    if kind in (_POSITIONAL_ONLY, _POSITIONAL_OR_KEYWORD):
                        if param.default is _empty:
                            if seen_default:
                                # No default for this parameter, but the
                                # previous parameter of had a default
                                msg = 'non-default argument follows default ' \
                                      'argument'
                                raise ValueError(msg)
                        else:
                            # There is a default for this parameter.
                            seen_default = True

                    if name in params:
                        msg = 'duplicate parameter name: {!r}'.format(name)
                        raise ValueError(msg)

                    params[name] = param
            else:
                params = OrderedDict((param.name, param) for param in parameters)

        self._parameters = types.MappingProxyType(params)
        self._return_annotation = return_annotation

    @classmethod
    def from_callable(cls, obj, *,
                      follow_wrapped=True, globals=None, locals=None, eval_str=False,
                      annotation_format=Format.VALUE):
        """Constructs Signature for the given callable object."""
        return _signature_from_callable(obj, sigcls=cls,
                                        follow_wrapper_chains=follow_wrapped,
                                        globals=globals, locals=locals, eval_str=eval_str,
                                        annotation_format=annotation_format)

    @property
    def parameters(self):
        return self._parameters

    @property
    def return_annotation(self):
        return self._return_annotation

    def replace(self, *, parameters=_void, return_annotation=_void):
        """Creates a customized copy of the Signature.
        Pass 'parameters' and/or 'return_annotation' arguments
        to override them in the new copy.
        """

        if parameters is _void:
            parameters = self.parameters.values()

        if return_annotation is _void:
            return_annotation = self._return_annotation

        return type(self)(parameters,
                          return_annotation=return_annotation)

    __replace__ = replace

    def _hash_basis(self):
        params = tuple(param for param in self.parameters.values()
                             if param.kind != _KEYWORD_ONLY)

        kwo_params = {param.name: param for param in self.parameters.values()
                                        if param.kind == _KEYWORD_ONLY}

        return params, kwo_params, self.return_annotation

    def __hash__(self):
        params, kwo_params, return_annotation = self._hash_basis()
        kwo_params = frozenset(kwo_params.values())
        return hash((params, kwo_params, return_annotation))

    def __eq__(self, other):
        if self is other:
            return True
        if not isinstance(other, Signature):
            return NotImplemented
        return self._hash_basis() == other._hash_basis()

    def _bind(self, args, kwargs, *, partial=False):
        """Private method. Don't use directly."""

        arguments = {}

        parameters = iter(self.parameters.values())
        parameters_ex = ()
        arg_vals = iter(args)

        pos_only_param_in_kwargs = []

        while True:
            # Let's iterate through the positional arguments and corresponding
            # parameters
            try:
                arg_val = next(arg_vals)
            except StopIteration:
                # No more positional arguments
                try:
                    param = next(parameters)
                except StopIteration:
                    # No more parameters. That's it. Just need to check that
                    # we have no `kwargs` after this while loop
                    break
                else:
                    if param.kind == _VAR_POSITIONAL:
                        # That's OK, just empty *args.  Let's start parsing
                        # kwargs
                        break
                    elif param.name in kwargs:
                        if param.kind == _POSITIONAL_ONLY:
                            if param.default is _empty:
                                msg = f'missing a required positional-only argument: {param.name!r}'
                                raise TypeError(msg)
                            # Raise a TypeError once we are sure there is no
                            # **kwargs param later.
                            pos_only_param_in_kwargs.append(param)
                            continue
                        parameters_ex = (param,)
                        break
                    elif (param.kind == _VAR_KEYWORD or
                                                param.default is not _empty):
                        # That's fine too - we have a default value for this
                        # parameter.  So, lets start parsing `kwargs`, starting
                        # with the current parameter
                        parameters_ex = (param,)
                        break
                    else:
                        # No default, not VAR_KEYWORD, not VAR_POSITIONAL,
                        # not in `kwargs`
                        if partial:
                            parameters_ex = (param,)
                            break
                        else:
                            if param.kind == _KEYWORD_ONLY:
                                argtype = ' keyword-only'
                            else:
                                argtype = ''
                            msg = 'missing a required{argtype} argument: {arg!r}'
                            msg = msg.format(arg=param.name, argtype=argtype)
                            raise TypeError(msg) from None
            else:
                # We have a positional argument to process
                try:
                    param = next(parameters)
                except StopIteration:
                    raise TypeError('too many positional arguments') from None
                else:
                    if param.kind in (_VAR_KEYWORD, _KEYWORD_ONLY):
                        # Looks like we have no parameter for this positional
                        # argument
                        raise TypeError(
                            'too many positional arguments') from None

                    if param.kind == _VAR_POSITIONAL:
                        # We have an '*args'-like argument, let's fill it with
                        # all positional arguments we have left and move on to
                        # the next phase
                        values = [arg_val]
                        values.extend(arg_vals)
                        arguments[param.name] = tuple(values)
                        break

                    if param.name in kwargs and param.kind != _POSITIONAL_ONLY:
                        raise TypeError(
                            'multiple values for argument {arg!r}'.format(
                                arg=param.name)) from None

                    arguments[param.name] = arg_val

        # Now, we iterate through the remaining parameters to process
        # keyword arguments
        kwargs_param = None
        for param in itertools.chain(parameters_ex, parameters):
            if param.kind == _VAR_KEYWORD:
                # Memorize that we have a '**kwargs'-like parameter
                kwargs_param = param
                continue

            if param.kind == _VAR_POSITIONAL:
                # Named arguments don't refer to '*args'-like parameters.
                # We only arrive here if the positional arguments ended
                # before reaching the last parameter before *args.
                continue

            param_name = param.name
            try:
                arg_val = kwargs.pop(param_name)
            except KeyError:
                # We have no value for this parameter.  It's fine though,
                # if it has a default value, or it is an '*args'-like
                # parameter, left alone by the processing of positional
                # arguments.
                if (not partial and param.kind != _VAR_POSITIONAL and
                                                    param.default is _empty):
                    raise TypeError('missing a required argument: {arg!r}'. \
                                    format(arg=param_name)) from None

            else:
                arguments[param_name] = arg_val

        if kwargs:
            if kwargs_param is not None:
                # Process our '**kwargs'-like parameter
                arguments[kwargs_param.name] = kwargs
            elif pos_only_param_in_kwargs:
                raise TypeError(
                    'got some positional-only arguments passed as '
                    'keyword arguments: {arg!r}'.format(
                        arg=', '.join(
                            param.name
                            for param in pos_only_param_in_kwargs
                        ),
                    ),
                )
            else:
                raise TypeError(
                    'got an unexpected keyword argument {arg!r}'.format(
                        arg=next(iter(kwargs))))

        return self._bound_arguments_cls(self, arguments)

    def bind(self, /, *args, **kwargs):
        """Get a BoundArguments object, that maps the passed `args`
        and `kwargs` to the function's signature.  Raises `TypeError`
        if the passed arguments can not be bound.
        """
        return self._bind(args, kwargs)

    def bind_partial(self, /, *args, **kwargs):
        """Get a BoundArguments object, that partially maps the
        passed `args` and `kwargs` to the function's signature.
        Raises `TypeError` if the passed arguments can not be bound.
        """
        return self._bind(args, kwargs, partial=True)

    def __reduce__(self):
        return (type(self),
                (tuple(self._parameters.values()),),
                {'_return_annotation': self._return_annotation})

    def __setstate__(self, state):
        self._return_annotation = state['_return_annotation']

    def __repr__(self):
        return '<{} {}>'.format(self.__class__.__name__, self)

    def __str__(self):
        return self.format()

    def format(self, *, max_width=None, quote_annotation_strings=True):
        """Create a string representation of the Signature object.

        If *max_width* integer is passed,
        signature will try to fit into the *max_width*.
        If signature is longer than *max_width*,
        all parameters will be on separate lines.

        If *quote_annotation_strings* is False, annotations
        in the signature are displayed without opening and closing quotation
        marks. This is useful when the signature was created with the
        STRING format or when ``from __future__ import annotations`` was used.
        """
        result = []
        render_pos_only_separator = False
        render_kw_only_separator = True
        for param in self.parameters.values():
            formatted = param._format(quote_annotation_strings=quote_annotation_strings)

            kind = param.kind

            if kind == _POSITIONAL_ONLY:
                render_pos_only_separator = True
            elif render_pos_only_separator:
                # It's not a positional-only parameter, and the flag
                # is set to 'True' (there were pos-only params before.)
                result.append('/')
                render_pos_only_separator = False

            if kind == _VAR_POSITIONAL:
                # OK, we have an '*args'-like parameter, so we won't need
                # a '*' to separate keyword-only arguments
                render_kw_only_separator = False
            elif kind == _KEYWORD_ONLY and render_kw_only_separator:
                # We have a keyword-only parameter to render and we haven't
                # rendered an '*args'-like parameter before, so add a '*'
                # separator to the parameters list ("foo(arg1, *, arg2)" case)
                result.append('*')
                # This condition should be only triggered once, so
                # reset the flag
                render_kw_only_separator = False

            result.append(formatted)

        if render_pos_only_separator:
            # There were only positional-only parameters, hence the
            # flag was not reset to 'False'
            result.append('/')

        rendered = '({})'.format(', '.join(result))
        if max_width is not None and len(rendered) > max_width:
            rendered = '(\n    {}\n)'.format(',\n    '.join(result))

        if self.return_annotation is not _empty:
            anno = formatannotation(self.return_annotation,
                                    quote_annotation_strings=quote_annotation_strings)
            rendered += ' -> {}'.format(anno)

        return rendered


# Reconstruct callable parameters from the runtime's public code fields.
# Source inspection: CPython v3.14.8 Lib/inspect.py; PSF License.
import os
import linecache
import tokenize
import importlib.machinery
modulesbyfile = {}
_filesbymodname = {}


def ismodule(object):
    """Return true if the object is a module."""
    return isinstance(object, types.ModuleType)


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


def istraceback(object):
    """Return true if the object is a traceback.

    Traceback objects provide these attributes:
        tb_frame        frame object at this level
        tb_lasti        index of last attempted instruction in bytecode
        tb_lineno       current line number in Python source code
        tb_next         next inner traceback object (called by this level)"""
    return isinstance(object, types.TracebackType)


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

# Callable signature dispatch: CPython v3.14.8 Lib/inspect.py; PSF License.
import ast
import itertools
import token
from annotationlib import get_annotations
_NonUserDefinedCallables = (types.WrapperDescriptorType,
                            types.MethodWrapperType,
                            types.ClassMethodDescriptorType,
                            types.BuiltinFunctionType)


def _signature_get_user_defined_method(cls, method_name, *, follow_wrapper_chains=True):
    """Private helper. Checks if ``cls`` has an attribute
    named ``method_name`` and returns it only if it is a
    pure python function.
    """
    if method_name == '__new__':
        meth = getattr(cls, method_name, None)
    else:
        meth = getattr_static(cls, method_name, None)
    if meth is None:
        return None

    # NOTE: The meth may wraps a non-user-defined callable.
    # In this case, we treat the meth as non-user-defined callable too.
    # (e.g. cls.__new__ generated by @warnings.deprecated)
    unwrapped_meth = None
    if follow_wrapper_chains:
        unwrapped_meth = unwrap(meth, stop=(lambda m: hasattr(m, "__signature__")
                                  or _signature_is_builtin(m)))

    if (isinstance(meth, _NonUserDefinedCallables)
          or isinstance(unwrapped_meth, _NonUserDefinedCallables)):
        # Once '__signature__' will be added to 'C'-level
        # callables, this check won't be necessary
        return None
    if method_name != '__new__':
        meth = _descriptor_get(meth, cls)
    return meth


def _signature_get_partial(wrapped_sig, partial, extra_args=()):
    """Private helper to calculate how 'wrapped_sig' signature will
    look like after applying a 'functools.partial' object (or alike)
    on it.
    """

    old_params = wrapped_sig.parameters
    new_params = OrderedDict(old_params.items())

    partial_args = partial.args or ()
    partial_keywords = partial.keywords or {}

    if extra_args:
        partial_args = extra_args + partial_args

    try:
        ba = wrapped_sig.bind_partial(*partial_args, **partial_keywords)
    except TypeError as ex:
        msg = 'partial object {!r} has incorrect arguments'.format(partial)
        raise ValueError(msg) from ex


    transform_to_kwonly = False
    for param_name, param in old_params.items():
        try:
            arg_value = ba.arguments[param_name]
        except KeyError:
            pass
        else:
            if param.kind is _POSITIONAL_ONLY:
                # If positional-only parameter is bound by partial,
                # it effectively disappears from the signature
                # However, if it is a Placeholder it is not removed
                # And also looses default value
                if arg_value is functools.Placeholder:
                    new_params[param_name] = param.replace(default=_empty)
                else:
                    new_params.pop(param_name)
                continue

            if param.kind is _POSITIONAL_OR_KEYWORD:
                if param_name in partial_keywords:
                    # This means that this parameter, and all parameters
                    # after it should be keyword-only (and var-positional
                    # should be removed). Here's why. Consider the following
                    # function:
                    #     foo(a, b, *args, c):
                    #         pass
                    #
                    # "partial(foo, a='spam')" will have the following
                    # signature: "(*, a='spam', b, c)". Because attempting
                    # to call that partial with "(10, 20)" arguments will
                    # raise a TypeError, saying that "a" argument received
                    # multiple values.
                    transform_to_kwonly = True
                    # Set the new default value
                    new_params[param_name] = param.replace(default=arg_value)
                else:
                    # was passed as a positional argument
                    # Do not pop if it is a Placeholder
                    #   also change kind to positional only
                    #   and remove default
                    if arg_value is functools.Placeholder:
                        new_param = param.replace(
                            kind=_POSITIONAL_ONLY,
                            default=_empty
                        )
                        new_params[param_name] = new_param
                    else:
                        new_params.pop(param_name)
                    continue

            if param.kind is _KEYWORD_ONLY:
                # Set the new default value
                new_params[param_name] = param.replace(default=arg_value)

        if transform_to_kwonly:
            assert param.kind is not _POSITIONAL_ONLY

            if param.kind is _POSITIONAL_OR_KEYWORD:
                new_param = new_params[param_name].replace(kind=_KEYWORD_ONLY)
                new_params[param_name] = new_param
                new_params.move_to_end(param_name)
            elif param.kind in (_KEYWORD_ONLY, _VAR_KEYWORD):
                new_params.move_to_end(param_name)
            elif param.kind is _VAR_POSITIONAL:
                new_params.pop(param.name)

    return wrapped_sig.replace(parameters=new_params.values())


def _signature_bound_method(sig):
    """Private helper to transform signatures for unbound
    functions to bound methods.
    """

    params = tuple(sig.parameters.values())

    if not params or params[0].kind in (_VAR_KEYWORD, _KEYWORD_ONLY):
        raise ValueError('invalid method signature')

    kind = params[0].kind
    if kind in (_POSITIONAL_OR_KEYWORD, _POSITIONAL_ONLY):
        # Drop first parameter:
        # '(p1, p2[, ...])' -> '(p2[, ...])'
        params = params[1:]
    else:
        if kind is not _VAR_POSITIONAL:
            # Unless we add a new parameter type we never
            # get here
            raise ValueError('invalid argument type')
        # It's a var-positional parameter.
        # Do nothing. '(*args[, ...])' -> '(*args[, ...])'

    return sig.replace(parameters=params)


def _signature_is_builtin(obj):
    """Private helper to test if `obj` is a callable that might
    support Argument Clinic's __text_signature__ protocol.
    """
    return (isbuiltin(obj) or
            ismethoddescriptor(obj) or
            isinstance(obj, _NonUserDefinedCallables) or
            # Can't test 'isinstance(type)' here, as it would
            # also be True for regular python classes.
            # Can't use the `in` operator here, as it would
            # invoke the custom __eq__ method.
            obj is type or obj is object)


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


def _signature_strip_non_python_syntax(signature):
    """
    Private helper function. Takes a signature in Argument Clinic's
    extended signature format.

    Returns a tuple of two things:
      * that signature re-rendered in standard Python syntax, and
      * the index of the "self" parameter (generally 0), or None if
        the function does not have a "self" parameter.
    """

    if not signature:
        return signature, None

    self_parameter = None

    lines = [l.encode('ascii') for l in signature.split('\n') if l]
    generator = iter(lines).__next__
    token_stream = tokenize.tokenize(generator)

    text = []
    add = text.append

    current_parameter = 0
    OP = token.OP
    ERRORTOKEN = token.ERRORTOKEN

    # token stream always starts with ENCODING token, skip it
    t = next(token_stream)
    assert t.type == tokenize.ENCODING

    for t in token_stream:
        type, string = t.type, t.string

        if type == OP:
            if string == ',':
                current_parameter += 1

        if (type == OP) and (string == '$'):
            assert self_parameter is None
            self_parameter = current_parameter
            continue

        add(string)
        if (string == ','):
            add(' ')
    clean_signature = ''.join(text).strip().replace("\n", "")
    return clean_signature, self_parameter


def _signature_fromstr(cls, obj, s, skip_bound_arg=True):
    """Private helper to parse content of '__text_signature__'
    and return a Signature based on it.
    """
    Parameter = cls._parameter_cls

    clean_signature, self_parameter = _signature_strip_non_python_syntax(s)
    program = "def foo" + clean_signature + ": pass"
    parts = [part.strip() for part in clean_signature[1:-1].split(',')]
    if parts and not parts[-1]:
        parts.pop()
    simple = True
    for part in parts:
        if part not in ('/', '*') and not part.lstrip('*').isidentifier():
            simple = False
            break
    if simple:
        # Parameter-only Clinic signatures need syntax validation, not a syntax tree.
        try:
            compile(program, '<signature>', 'exec')
        except SyntaxError:
            raise ValueError("{!r} builtin has invalid signature".format(obj)) from None
        parameters = []
        kind = Parameter.POSITIONAL_OR_KEYWORD
        for part in parts:
            if part == '/':
                parameters = [p.replace(kind=Parameter.POSITIONAL_ONLY) for p in parameters]
            elif part == '*':
                kind = Parameter.KEYWORD_ONLY
            elif part.startswith('**'):
                parameters.append(Parameter(part[2:], Parameter.VAR_KEYWORD))
            elif part.startswith('*'):
                parameters.append(Parameter(part[1:], Parameter.VAR_POSITIONAL))
                kind = Parameter.KEYWORD_ONLY
            else:
                parameters.append(Parameter(part, kind))
        if self_parameter is not None:
            bound = getattr(obj, '__self__', None)
            if bound is not None and (ismodule(bound) or skip_bound_arg):
                parameters.pop(0)
            elif parameters:
                parameters[0] = parameters[0].replace(kind=Parameter.POSITIONAL_ONLY)
        return cls(parameters, return_annotation=cls.empty)

    try:
        module = ast.parse(program)
    except SyntaxError:
        module = None

    if not isinstance(module, ast.Module):
        raise ValueError("{!r} builtin has invalid signature".format(obj))

    f = module.body[0]

    parameters = []
    empty = Parameter.empty

    module = None
    module_dict = {}

    module_name = getattr(obj, '__module__', None)
    if not module_name:
        objclass = getattr(obj, '__objclass__', None)
        module_name = getattr(objclass, '__module__', None)

    if module_name:
        module = sys.modules.get(module_name, None)
        if module:
            module_dict = module.__dict__
    sys_module_dict = sys.modules.copy()

    def parse_name(node):
        assert isinstance(node, ast.arg)
        if node.annotation is not None:
            raise ValueError("Annotations are not currently supported")
        return node.arg

    def wrap_value(s):
        try:
            value = eval(s, module_dict)
        except NameError:
            try:
                value = eval(s, sys_module_dict)
            except NameError:
                raise ValueError

        if isinstance(value, (str, int, float, bytes, bool, type(None))):
            return ast.Constant(value)
        raise ValueError

    class RewriteSymbolics(ast.NodeTransformer):
        def visit_Attribute(self, node):
            a = []
            n = node
            while isinstance(n, ast.Attribute):
                a.append(n.attr)
                n = n.value
            if not isinstance(n, ast.Name):
                raise ValueError
            a.append(n.id)
            value = ".".join(reversed(a))
            return wrap_value(value)

        def visit_Name(self, node):
            if not isinstance(node.ctx, ast.Load):
                raise ValueError()
            return wrap_value(node.id)

        def visit_BinOp(self, node):
            # Support constant folding of a couple simple binary operations
            # commonly used to define default values in text signatures
            left = self.visit(node.left)
            right = self.visit(node.right)
            if not isinstance(left, ast.Constant) or not isinstance(right, ast.Constant):
                raise ValueError
            if isinstance(node.op, ast.Add):
                return ast.Constant(left.value + right.value)
            elif isinstance(node.op, ast.Sub):
                return ast.Constant(left.value - right.value)
            elif isinstance(node.op, ast.BitOr):
                return ast.Constant(left.value | right.value)
            raise ValueError

    def p(name_node, default_node, default=empty):
        name = parse_name(name_node)
        if default_node and default_node is not _empty:
            try:
                default_node = RewriteSymbolics().visit(default_node)
                default = ast.literal_eval(default_node)
            except ValueError:
                raise ValueError("{!r} builtin has invalid signature".format(obj)) from None
        parameters.append(Parameter(name, kind, default=default, annotation=empty))

    # non-keyword-only parameters
    total_non_kw_args = len(f.args.posonlyargs) + len(f.args.args)
    required_non_kw_args = total_non_kw_args - len(f.args.defaults)
    defaults = itertools.chain(itertools.repeat(None, required_non_kw_args), f.args.defaults)

    kind = Parameter.POSITIONAL_ONLY
    for (name, default) in zip(f.args.posonlyargs, defaults):
        p(name, default)

    kind = Parameter.POSITIONAL_OR_KEYWORD
    for (name, default) in zip(f.args.args, defaults):
        p(name, default)

    # *args
    if f.args.vararg:
        kind = Parameter.VAR_POSITIONAL
        p(f.args.vararg, empty)

    # keyword-only arguments
    kind = Parameter.KEYWORD_ONLY
    for name, default in zip(f.args.kwonlyargs, f.args.kw_defaults):
        p(name, default)

    # **kwargs
    if f.args.kwarg:
        kind = Parameter.VAR_KEYWORD
        p(f.args.kwarg, empty)

    if self_parameter is not None:
        # Possibly strip the bound argument:
        #    - We *always* strip first bound argument if
        #      it is a module.
        #    - We don't strip first bound argument if
        #      skip_bound_arg is False.
        assert parameters
        _self = getattr(obj, '__self__', None)
        self_isbound = _self is not None
        self_ismodule = ismodule(_self)
        if self_isbound and (self_ismodule or skip_bound_arg):
            parameters.pop(0)
        else:
            # for builtins, self parameter is always positional-only!
            p = parameters[0].replace(kind=Parameter.POSITIONAL_ONLY)
            parameters[0] = p

    return cls(parameters, return_annotation=cls.empty)


def _signature_from_builtin(cls, func, skip_bound_arg=True):
    """Private helper function to get signature for
    builtin callables.
    """

    if not _signature_is_builtin(func):
        raise TypeError("{!r} is not a Python builtin "
                        "function".format(func))

    s = getattr(func, "__text_signature__", None)
    if not s:
        raise ValueError("no signature found for builtin {!r}".format(func))

    return _signature_fromstr(cls, func, s, skip_bound_arg)


def _signature_from_function(cls, func, skip_bound_arg=True,
                             globals=None, locals=None, eval_str=False,
                             *, annotation_format=Format.VALUE):
    """Private helper: constructs Signature for the given python function."""

    is_duck_function = False
    if not isfunction(func):
        if _signature_is_functionlike(func):
            is_duck_function = True
        else:
            # If it's not a pure Python function, and not a duck type
            # of pure function (Cython and Mock functions, for instance), then:
            raise TypeError('{!r} is not a Python function'.format(func))

    s = getattr(func, "__text_signature__", None)
    if s:
        return _signature_fromstr(cls, func, s, skip_bound_arg)

    Parameter = cls._parameter_cls

    # Parameter information.
    func_code = func.__code__
    pos_count = func_code.co_argcount
    arg_names = func_code.co_varnames
    posonly_count = func_code.co_posonlyargcount
    positional = arg_names[:pos_count]
    keyword_only_count = func_code.co_kwonlyargcount
    keyword_only = arg_names[pos_count:pos_count + keyword_only_count]
    annotations = get_annotations(func, globals=globals, locals=locals, eval_str=eval_str,
                                  format=annotation_format)
    defaults = func.__defaults__
    kwdefaults = func.__kwdefaults__

    if defaults:
        pos_default_count = len(defaults)
    else:
        pos_default_count = 0

    parameters = []

    non_default_count = pos_count - pos_default_count
    posonly_left = posonly_count

    # Non-keyword-only parameters w/o defaults.
    for name in positional[:non_default_count]:
        kind = _POSITIONAL_ONLY if posonly_left else _POSITIONAL_OR_KEYWORD
        annotation = annotations.get(name, _empty)
        parameters.append(Parameter(name, annotation=annotation,
                                    kind=kind))
        if posonly_left:
            posonly_left -= 1

    # ... w/ defaults.
    for offset, name in enumerate(positional[non_default_count:]):
        kind = _POSITIONAL_ONLY if posonly_left else _POSITIONAL_OR_KEYWORD
        annotation = annotations.get(name, _empty)
        parameters.append(Parameter(name, annotation=annotation,
                                    kind=kind,
                                    default=defaults[offset]))
        if posonly_left:
            posonly_left -= 1

    # *args
    if func_code.co_flags & CO_VARARGS:
        name = arg_names[pos_count + keyword_only_count]
        annotation = annotations.get(name, _empty)
        parameters.append(Parameter(name, annotation=annotation,
                                    kind=_VAR_POSITIONAL))

    # Keyword-only parameters.
    for name in keyword_only:
        default = _empty
        if kwdefaults is not None:
            default = kwdefaults.get(name, _empty)

        annotation = annotations.get(name, _empty)
        parameters.append(Parameter(name, annotation=annotation,
                                    kind=_KEYWORD_ONLY,
                                    default=default))
    # **kwargs
    if func_code.co_flags & CO_VARKEYWORDS:
        index = pos_count + keyword_only_count
        if func_code.co_flags & CO_VARARGS:
            index += 1

        name = arg_names[index]
        annotation = annotations.get(name, _empty)
        parameters.append(Parameter(name, annotation=annotation,
                                    kind=_VAR_KEYWORD))

    # Is 'func' is a pure Python function - don't validate the
    # parameters list (for correct order and defaults), it should be OK.
    return cls(parameters,
               return_annotation=annotations.get('return', _empty),
               __validate_parameters__=is_duck_function)


def _descriptor_get(descriptor, obj):
    if isclass(descriptor):
        return descriptor
    get = getattr(type(descriptor), '__get__', _sentinel)
    if get is _sentinel:
        return descriptor
    return get(descriptor, obj, type(obj))


def _signature_from_callable(obj, *,
                             follow_wrapper_chains=True,
                             skip_bound_arg=True,
                             globals=None,
                             locals=None,
                             eval_str=False,
                             sigcls,
                             annotation_format=Format.VALUE):

    """Private helper function to get signature for arbitrary
    callable objects.
    """

    _get_signature_of = functools.partial(_signature_from_callable,
                                follow_wrapper_chains=follow_wrapper_chains,
                                skip_bound_arg=skip_bound_arg,
                                globals=globals,
                                locals=locals,
                                sigcls=sigcls,
                                eval_str=eval_str,
                                annotation_format=annotation_format)

    if not callable(obj):
        raise TypeError('{!r} is not a callable object'.format(obj))

    if isinstance(obj, types.MethodType):
        # In this case we skip the first parameter of the underlying
        # function (usually `self` or `cls`).
        sig = _get_signature_of(obj.__func__)

        if skip_bound_arg:
            return _signature_bound_method(sig)
        else:
            return sig

    # Was this function wrapped by a decorator?
    if follow_wrapper_chains:
        # Unwrap until we find an explicit signature or a MethodType (which will be
        # handled explicitly below).
        obj = unwrap(obj, stop=(lambda f: hasattr(f, "__signature__")
                                or isinstance(f, types.MethodType)))
        if isinstance(obj, types.MethodType):
            # If the unwrapped object is a *method*, we might want to
            # skip its first parameter (self).
            # See test_signature_wrapped_bound_method for details.
            return _get_signature_of(obj)

    try:
        sig = obj.__signature__
    except AttributeError:
        pass
    else:
        if sig is not None:
            if not isinstance(sig, Signature):
                raise TypeError(
                    'unexpected object {!r} in __signature__ '
                    'attribute'.format(sig))
            return sig

    try:
        partialmethod = obj.__partialmethod__
    except AttributeError:
        pass
    else:
        if isinstance(partialmethod, functools.partialmethod):
            # Unbound partialmethod (see functools.partialmethod)
            # This means, that we need to calculate the signature
            # as if it's a regular partial object, but taking into
            # account that the first positional argument
            # (usually `self`, or `cls`) will not be passed
            # automatically (as for boundmethods)

            wrapped_sig = _get_signature_of(partialmethod.func)

            sig = _signature_get_partial(wrapped_sig, partialmethod, (None,))
            first_wrapped_param = tuple(wrapped_sig.parameters.values())[0]
            if first_wrapped_param.kind is Parameter.VAR_POSITIONAL:
                # First argument of the wrapped callable is `*args`, as in
                # `partialmethod(lambda *args)`.
                return sig
            else:
                sig_params = tuple(sig.parameters.values())
                assert (not sig_params or
                        first_wrapped_param is not sig_params[0])
                # If there were placeholders set,
                #   first param is transformed to positional only
                if partialmethod.args.count(functools.Placeholder):
                    first_wrapped_param = first_wrapped_param.replace(
                        kind=Parameter.POSITIONAL_ONLY)
                new_params = (first_wrapped_param,) + sig_params
                return sig.replace(parameters=new_params)

    if isinstance(obj, functools.partial):
        wrapped_sig = _get_signature_of(obj.func)
        return _signature_get_partial(wrapped_sig, obj)

    if isfunction(obj) or _signature_is_functionlike(obj):
        # If it's a pure Python function, or an object that is duck type
        # of a Python function (Cython and Mock functions, for instance), then:
        return _signature_from_function(sigcls, obj,
                                        skip_bound_arg=skip_bound_arg,
                                        globals=globals, locals=locals, eval_str=eval_str,
                                        annotation_format=annotation_format)

    if _signature_is_builtin(obj):
        return _signature_from_builtin(sigcls, obj,
                                       skip_bound_arg=skip_bound_arg)

    if isinstance(obj, type):
        # obj is a class or a metaclass

        # First, let's see if it has an overloaded __call__ defined
        # in its metaclass
        call = _signature_get_user_defined_method(
            type(obj),
            '__call__',
            follow_wrapper_chains=follow_wrapper_chains,
        )
        if call is not None:
            return _get_signature_of(call)

        # NOTE: The user-defined method can be a function with a thin wrapper
        # around object.__new__ (e.g., generated by `@warnings.deprecated`)
        new = _signature_get_user_defined_method(
            obj,
            '__new__',
            follow_wrapper_chains=follow_wrapper_chains,
        )
        init = _signature_get_user_defined_method(
            obj,
            '__init__',
            follow_wrapper_chains=follow_wrapper_chains,
        )

        # Go through the MRO and see if any class has user-defined
        # pure Python __new__ or __init__ method
        for base in obj.__mro__:
            # Now we check if the 'obj' class has an own '__new__' method
            if new is not None and '__new__' in base.__dict__:
                sig = _get_signature_of(new)
                if skip_bound_arg:
                    sig = _signature_bound_method(sig)
                return sig
            # or an own '__init__' method
            elif init is not None and '__init__' in base.__dict__:
                return _get_signature_of(init)

        # At this point we know, that `obj` is a class, with no user-
        # defined '__init__', '__new__', or class-level '__call__'

        for base in obj.__mro__[:-1]:
            # Since '__text_signature__' is implemented as a
            # descriptor that extracts text signature from the
            # class docstring, if 'obj' is derived from a builtin
            # class, its own '__text_signature__' may be 'None'.
            # Therefore, we go through the MRO (except the last
            # class in there, which is 'object') to find the first
            # class with non-empty text signature.
            try:
                text_sig = base.__text_signature__
            except AttributeError:
                pass
            else:
                if text_sig:
                    # If 'base' class has a __text_signature__ attribute:
                    # return a signature based on it
                    return _signature_fromstr(sigcls, base, text_sig)

        # No '__text_signature__' was found for the 'obj' class.
        # Last option is to check if its '__init__' is
        # object.__init__ or type.__init__.
        if type not in obj.__mro__:
            obj_init = obj.__init__
            obj_new = obj.__new__
            if follow_wrapper_chains:
                obj_init = unwrap(obj_init)
                obj_new = unwrap(obj_new)
            # We have a class (not metaclass), but no user-defined
            # __init__ or __new__ for it
            if obj_init is object.__init__ and obj_new is object.__new__:
                # Return a signature of 'object' builtin.
                return sigcls.from_callable(object)
            else:
                raise ValueError(
                    'no signature found for builtin type {!r}'.format(obj))

    else:
        # An object with __call__
        call = getattr_static(type(obj), '__call__', None)
        if call is not None:
            try:
                text_sig = obj.__text_signature__
            except AttributeError:
                pass
            else:
                if text_sig:
                    return _signature_fromstr(sigcls, obj, text_sig)
            call = _descriptor_get(call, obj)
            return _get_signature_of(call)

    raise ValueError('callable {!r} is not supported by signature'.format(obj))



def isbuiltin(object):
    """Return true if the object is a built-in function or method.

    Built-in functions and methods provide these attributes:
        __doc__         documentation string
        __name__        original name of this function or method
        __self__        instance to which a method is bound, or None"""
    return isinstance(object, types.BuiltinFunctionType)

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

# Coroutine and generator objects: CPython v3.14.8 Lib/inspect.py; PSF License.
def iscoroutine(object):
    """Return true if the object is a coroutine."""
    return isinstance(object, types.CoroutineType)

# Coroutine and generator objects: CPython v3.14.8 Lib/inspect.py; PSF License.
def isgenerator(object):
    """Return true if the object is a generator.

    Generator objects provide these attributes:
        gi_code         code object
        gi_frame        frame object or possibly None once the generator has
                        been exhausted
        gi_running      set to 1 when generator is executing, 0 otherwise
        gi_suspended    set to 1 when the generator is suspended at a yield point, 0 otherwise
        gi_yieldfrom    object being iterated by yield from or None

        __iter__()      defined to support iteration over container
        close()         raises a new GeneratorExit exception inside the
                        generator to terminate the iteration
        send()          resumes the generator and "sends" a value that becomes
                        the result of the current yield-expression
        throw()         used to raise an exception inside the generator"""
    return isinstance(object, types.GeneratorType)

# Coroutine and generator objects: CPython v3.14.8 Lib/inspect.py; PSF License.
def isasyncgen(object):
    """Return true if the object is an asynchronous generator."""
    return isinstance(object, types.AsyncGeneratorType)
