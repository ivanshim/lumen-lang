# Names for the kinds of thing a program builds, and the few small
# kinds that can be built here in Python.
#
# Some callable and frame constructors below remain placeholders. Code
# and generator kinds are obtained from values made by the runtime.

NoneType = type(None)


class FunctionType:
    def __init__(self, code, globals):
        self.__code__ = code
        self.__globals__ = globals

    def __call__(self):
        return eval(self.__code__, self.__globals__)


LambdaType = FunctionType


CodeType = type((lambda: None).__code__)


class BuiltinFunctionType:
    pass


BuiltinMethodType = BuiltinFunctionType


class WrapperDescriptorType:
    pass


class MethodWrapperType:
    pass


class MethodDescriptorType:
    pass


class ClassMethodDescriptorType:
    pass


class GetSetDescriptorType:
    pass


class MemberDescriptorType:
    pass


def __generator_probe():
    yield


GeneratorType = type(__generator_probe())
del __generator_probe


async def _coroutine_kind_probe():
    pass
_coroutine_kind_value = _coroutine_kind_probe()
CoroutineType = type(_coroutine_kind_value)
_coroutine_kind_value.close()
del _coroutine_kind_value, _coroutine_kind_probe

async def _async_generator_kind_probe():
    yield
AsyncGeneratorType = type(_async_generator_kind_probe())
del _async_generator_kind_probe


class FrameType:
    pass


class TracebackType:
    pass


class EllipsisType:
    pass


class NotImplementedType:
    pass


class UnionType:
    pass


# A cell holds one value, or none at all, and reading an empty one is an
# error rather than an answer.
class CellType:
    def __init__(self, *contents):
        if len(contents) > 1:
            raise TypeError('cell expected at most 1 argument, got ' + str(len(contents)))
        self._contents = list(contents)

    @property
    def cell_contents(self):
        if len(self._contents) == 0:
            raise ValueError('Cell is empty')
        return self._contents[0]

    def __repr__(self):
        if len(self._contents) == 0:
            return '<cell: empty>'
        return '<cell: ' + repr(self._contents[0]) + '>'


# A function bound to a thing, called as one: reading through it reads the
# function, and it keeps no namespace to write anything else into.
class method:
    __slots__ = ('__func__', '__self__')

    def __init__(self, function, instance):
        self.__func__ = function
        self.__self__ = instance

    def __call__(self, *args, **keywords):
        return self.__func__(self.__self__, *args, **keywords)

    def __getattr__(self, name):
        return getattr(self.__func__, name)

    def __eq__(self, other):
        if isinstance(other, MethodType):
            return self.__func__ == other.__func__ and self.__self__ is other.__self__
        return NotImplemented

    def __repr__(self):
        return '<bound method of ' + repr(self.__self__) + '>'


MethodType = method


# A place to hang names on, which is all a module is from here.
class ModuleType:
    def __init__(self, name, doc=None):
        self.__name__ = name
        self.__doc__ = doc

    def __repr__(self):
        return "<module '" + str(getattr(self, '__name__', '?')) + "'>"


# A reading of a mapping that cannot be written through.
class MappingProxyType:
    def __init__(self, mapping):
        self._mapping = mapping

    def __getitem__(self, key):
        return self._mapping[key]

    def __len__(self):
        return len(self._mapping)

    def __iter__(self):
        return iter(list(self._mapping))

    def __contains__(self, key):
        return key in self._mapping

    def get(self, key, default=None):
        if key in self._mapping:
            return self._mapping[key]
        return default

    def keys(self):
        return list(self._mapping)

    def values(self):
        gathered = []
        for key in list(self._mapping):
            gathered.append(self._mapping[key])
        return gathered

    def items(self):
        gathered = []
        for key in list(self._mapping):
            gathered.append((key, self._mapping[key]))
        return gathered

    def copy(self):
        copied = {}
        for key in list(self._mapping):
            copied[key] = self._mapping[key]
        return copied

    def __eq__(self, other):
        if isinstance(other, MappingProxyType):
            return self.copy() == other.copy()
        return self.copy() == other

    def __repr__(self):
        return 'mappingproxy(' + repr(self._mapping) + ')'


# A bag of named values, compared by the names it carries.
class SimpleNamespace:
    def __init__(self, **keywords):
        for name in list(keywords):
            setattr(self, name, keywords[name])

    def _fields(self):
        gathered = {}
        for name in list(self.__dict__):
            gathered[name] = self.__dict__[name]
        return gathered

    def __eq__(self, other):
        if isinstance(other, SimpleNamespace):
            return self._fields() == other._fields()
        return NotImplemented

    def __ne__(self, other):
        answer = self.__eq__(other)
        if answer is NotImplemented:
            return answer
        return not answer

    def __repr__(self):
        fields = self._fields()
        shown = []
        for name in sorted(list(fields)):
            shown.append(name + '=' + repr(fields[name]))
        return 'namespace(' + ', '.join(shown) + ')'


# What a kind is called. A builtin kind answers to __name__ but is not
# found by getattr, so the name is asked for directly.
def _kind_name(kind):
    try:
        return str(kind.__name__)
    except BaseException:
        return repr(kind)


# A kind named with the kinds it was given, as list[int] is.
class GenericAlias:
    def __init__(self, origin, args):
        self.__origin__ = origin
        if isinstance(args, tuple):
            self.__args__ = args
        else:
            self.__args__ = (args,)

    @property
    def __parameters__(self):
        parameters = []
        for arg in self.__args__:
            nested = getattr(arg, '__parameters__', ())
            if type(arg).__name__ in ('TypeVar', 'TypeVarTuple', 'ParamSpec'):
                nested = (arg,)
            for parameter in nested:
                if parameter not in parameters:
                    parameters.append(parameter)
        return tuple(parameters)

    def __call__(self, *args, **keywords):
        return self.__origin__(*args, **keywords)

    def __eq__(self, other):
        if isinstance(other, GenericAlias):
            return self.__origin__ is other.__origin__ and self.__args__ == other.__args__
        return NotImplemented

    def __repr__(self):
        shown = []
        for given in self.__args__:
            shown.append(_kind_name(given))
        return _kind_name(self.__origin__) + '[' + ', '.join(shown) + ']'

    def __iter__(self):
        return _GenericAliasIterator(self)


class _UnpackedGenericAlias:
    def __init__(self, alias):
        self.__origin__ = alias.__origin__
        self.__args__ = alias.__args__
        self.__unpacked__ = True
        self._alias = alias

    def __repr__(self):
        return '*' + repr(self._alias)

    def __eq__(self, other):
        if isinstance(other, _UnpackedGenericAlias):
            return self._alias == other._alias
        return NotImplemented


class _GenericAliasIterator:
    def __init__(self, alias):
        self.alias = alias
        self.done = False

    def __iter__(self):
        return self

    def __next__(self):
        if self.done:
            raise StopIteration
        self.done = True
        return _UnpackedGenericAlias(self.alias)

    def __reduce__(self):
        import builtins
        factory = builtins.__dict__['iter']
        if self.done:
            return (factory, ((),))
        return (factory, (self.alias,))


# A member that answers one way on an instance and another on the class.
class DynamicClassAttribute:
    def __init__(self, fget=None, fset=None, fdel=None, doc=None):
        self.fget = fget
        self.fset = fset
        self.fdel = fdel
        self.__doc__ = doc if doc is not None else getattr(fget, '__doc__', None)
        self.overwrite_doc = doc is None

    def __get__(self, instance, owner=None):
        if instance is None:
            raise AttributeError('dynamic class attribute')
        if self.fget is None:
            raise AttributeError('unreadable attribute')
        return self.fget(instance)

    def __set__(self, instance, value):
        if self.fset is None:
            raise AttributeError("can't set attribute")
        self.fset(instance, value)

    def __delete__(self, instance):
        if self.fdel is None:
            raise AttributeError("can't delete attribute")
        self.fdel(instance)

    def getter(self, fget):
        return DynamicClassAttribute(fget, self.fset, self.fdel, self.__doc__)

    def setter(self, fset):
        return DynamicClassAttribute(self.fget, fset, self.fdel, self.__doc__)

    def deleter(self, fdel):
        return DynamicClassAttribute(self.fget, self.fset, fdel, self.__doc__)


__all__ = ['NoneType', 'FunctionType', 'LambdaType', 'CodeType',
           'CellType', 'MethodType', 'BuiltinFunctionType',
           'BuiltinMethodType', 'WrapperDescriptorType',
           'MethodWrapperType', 'MethodDescriptorType',
           'ClassMethodDescriptorType', 'GetSetDescriptorType',
           'MemberDescriptorType', 'GeneratorType', 'CoroutineType',
           'AsyncGeneratorType', 'FrameType', 'TracebackType',
           'EllipsisType', 'NotImplementedType', 'UnionType',
           'ModuleType', 'MappingProxyType', 'SimpleNamespace',
           'GenericAlias', 'DynamicClassAttribute']

# Source: CPython 3b564385e4c9, Lib/types.py. PSF License.
# Function recognition uses the native function kind rather than its constructor.
class _GeneratorWrapper:
    def __init__(self, gen):
        self.__wrapped = gen
        self.__isgen = gen.__class__ is GeneratorType
        self.__name__ = getattr(gen, '__name__', None)
        self.__qualname__ = getattr(gen, '__qualname__', None)
    def send(self, val):
        return self.__wrapped.send(val)
    def throw(self, tp, *rest):
        return self.__wrapped.throw(tp, *rest)
    def close(self):
        return self.__wrapped.close()
    @property
    def gi_code(self):
        return self.__wrapped.gi_code
    @property
    def gi_frame(self):
        return self.__wrapped.gi_frame
    @property
    def gi_running(self):
        return self.__wrapped.gi_running
    @property
    def gi_yieldfrom(self):
        return self.__wrapped.gi_yieldfrom
    @property
    def gi_suspended(self):
        return self.__wrapped.gi_suspended
    @property
    def gi_state(self):
        return self.__wrapped.gi_state
    @property
    def cr_state(self):
        return self.__wrapped.gi_state.replace('GEN_', 'CORO_')
    cr_code = gi_code
    cr_frame = gi_frame
    cr_running = gi_running
    cr_await = gi_yieldfrom
    cr_suspended = gi_suspended
    def __next__(self):
        return next(self.__wrapped)
    def __iter__(self):
        if self.__isgen:
            return self.__wrapped
        return self
    __await__ = __iter__

def coroutine(func):
    """Convert regular generator function to a coroutine."""

    if not callable(func):
        raise TypeError('types.coroutine() expects a callable')

    if (func.__class__ is type(lambda: None) and
        getattr(func, '__code__', None).__class__ is CodeType):

        co_flags = func.__code__.co_flags

        # Check if 'func' is a coroutine function.
        # (0x180 == CO_COROUTINE | CO_ITERABLE_COROUTINE)
        if co_flags & 0x180:
            return func

        # Check if 'func' is a generator function.
        # (0x20 == CO_GENERATOR)
        if co_flags & 0x20:
            co = func.__code__
            # 0x100 == CO_ITERABLE_COROUTINE
            func.__code__ = co.replace(co_flags=co.co_flags | 0x100)
            return func

    # The following code is primarily to support functions that
    # return generator-like objects (for instance generators
    # compiled with Cython).

    # Delay functools and _collections_abc import for speeding up types import.
    import functools
    import _collections_abc
    @functools.wraps(func)
    def wrapped(*args, **kwargs):
        coro = func(*args, **kwargs)
        if (coro.__class__ is CoroutineType or
            coro.__class__ is GeneratorType and coro.gi_code.co_flags & 0x100):
            # 'coro' is a native coroutine object or an iterable coroutine
            return coro
        if (isinstance(coro, _collections_abc.Generator) and
            not isinstance(coro, _collections_abc.Coroutine)):
            # 'coro' is either a pure Python generator iterator, or it
            # implements collections.abc.Generator (and does not implement
            # collections.abc.Coroutine).
            return _GeneratorWrapper(coro)
        # 'coro' is either an instance of collections.abc.Coroutine or
        # some other object -- pass it through.
        return coro

    return wrapped

__all__.append("coroutine")
