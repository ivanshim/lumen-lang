# Bindings and constructors for CPython Modules/_typesmodule.c.
# PSF License.
def _cell_factory():
    a = 1
    def f():
        nonlocal a
    return f.__closure__[0]
CellType = type(_cell_factory())
del _cell_factory


class _MethodSample:
    def method(self): pass
MethodType = type(_MethodSample().method)
del _MethodSample


# ModuleType is the runtime's native module class, including its readonly
# namespace descriptor and subclass initialization protocol.
import sys
ModuleType = type(sys)


GenericAlias = type(list[int])

# These kinds are the runtime's own; the names here are the kinds the
# pure fallback in types.py reads off the same values.
# A reading of a mapping that cannot be written through. Every question
# goes to the mapping held, so a mapping's own methods keep their say.
class MappingProxyType:
    __slots__ = ('_mapping',)

    def __init__(self, mapping):
        if not hasattr(mapping, 'keys') or not hasattr(mapping, '__getitem__') or isinstance(mapping, (list, tuple)):
            raise TypeError('mappingproxy() argument must be a mapping, not ' + type(mapping).__name__)
        self._mapping = mapping

    def __getitem__(self, key):
        return self._mapping[key]

    def __len__(self):
        return len(self._mapping)

    def __iter__(self):
        return iter(self._mapping)

    def __reversed__(self):
        return reversed(self._mapping)

    def __contains__(self, key):
        return key in self._mapping

    def __hash__(self):
        return hash(self._mapping)

    def get(self, key, default=None):
        return self._mapping.get(key, default)

    def keys(self):
        return self._mapping.keys()

    def values(self):
        return self._mapping.values()

    def items(self):
        return self._mapping.items()

    def copy(self):
        return self._mapping.copy()

    def __eq__(self, other):
        return self._mapping == other

    def __ne__(self, other):
        return self._mapping != other

    def __lt__(self, other):
        return self._mapping < other

    def __le__(self, other):
        return self._mapping <= other

    def __gt__(self, other):
        return self._mapping > other

    def __ge__(self, other):
        return self._mapping >= other

    # A reading stands for the mapping it reads when the union sign
    # reaches it, on either side. A reading cannot be written through
    # with the in-place sign.
    def __or__(self, other):
        right = other._mapping if isinstance(other, MappingProxyType) else other
        return self._mapping | right

    def __ror__(self, other):
        left = other._mapping if isinstance(other, MappingProxyType) else other
        return left | self._mapping

    def __ior__(self, other):
        raise TypeError("'|=' is not supported by mappingproxy; use '|' instead")

    def __class_getitem__(cls, item):
        return GenericAlias(cls, item)

    def __str__(self):
        return str(self._mapping)

    def __repr__(self):
        return 'mappingproxy(' + repr(self._mapping) + ')'

    # The slot holding the mapping is no part of what a reading offers.
    def __dir__(self):
        return [name for name in object.__dir__(self) if name not in ('_mapping', '__module__', '__slots__')]


# The reference names this kind mappingproxy, and types.MappingProxyType
# is that kind, so the name a type prints is the reference's.
MappingProxyType.__name__ = 'mappingproxy'
MappingProxyType.__qualname__ = 'mappingproxy'
MappingProxyType.__module__ = 'builtins'
SimpleNamespace = type(sys.implementation)
UnionType = type(int | str)


# What a built union (int | str) answers when it is subscripted, pickled
# or hashed; the runtime hands the union itself to each.
def _union_getitem(union, item):
    import typing
    params = union.__parameters__
    if not params:
        raise TypeError(repr(union) + ' is not a generic class')
    if not isinstance(item, tuple):
        item = (item,)
    if len(item) != len(params):
        few = 'few' if len(item) < len(params) else 'many'
        raise TypeError('Too %s arguments for %r; actual %d, expected %d' % (few, union, len(item), len(params)))
    chosen = [(p, p.__typing_subst__(v)) for p, v in zip(params, item)]
    new = []
    for arg in union.__args__:
        found = [v for p, v in chosen if p is arg]
        if found:
            new.append(found[0])
            continue
        inner = getattr(arg, '__parameters__', ())
        if inner:
            new.append(arg[tuple([v for p in inner for q, v in chosen if q is p])])
        else:
            new.append(arg)
    return typing.Union[tuple(new)]


def _union_reduce(union):
    import operator, typing
    return (operator.getitem, (typing.Union, union.__args__))


def _union_eq(union, other):
    if type(other) is not type(union):
        return NotImplemented
    mine, theirs = union.__args__, other.__args__
    return (all([any([x == y for y in theirs]) for x in mine])
            and all([any([y == x for x in mine]) for y in theirs]))


def _union_hash(union):
    return hash(frozenset(union.__args__))


# What a subscripted kind (list[int]) answers when it is pickled or hashed.
def _alias_reduce(alias):
    return (type(alias), (alias.__origin__, alias.__args__))


def _alias_hash(alias):
    return hash(alias.__origin__) ^ hash(alias.__args__)


# A type variable (or parameter spec, alias) is pickled and copied as its name.
def _name_reduce(thing):
    return thing.__name__


def _nodefault_reduce(thing):
    return 'NoDefault'


NoneType = type(None)
FunctionType = type(lambda: None)
LambdaType = FunctionType
CodeType = type((lambda: None).__code__)
BuiltinFunctionType = type(len)
BuiltinMethodType = BuiltinFunctionType
EllipsisType = type(Ellipsis)
NotImplementedType = type(NotImplemented)
WrapperDescriptorType = type(object.__init__)
MethodWrapperType = type(object().__str__)
MethodDescriptorType = type(str.join)
ClassMethodDescriptorType = type(dict.__dict__['fromkeys'])
GetSetDescriptorType = type(int.real)
MemberDescriptorType = type(complex.real)
def _generator(): yield None
GeneratorType = type(_generator())
del _generator
async def _coroutine(): return None
_coro = _coroutine()
CoroutineType = type(_coro)
_coro.close()
del _coroutine, _coro
async def _async_generator(): yield None
AsyncGeneratorType = type(_async_generator())
del _async_generator
try:
    raise TypeError
except TypeError as _exc:
    TracebackType = type(_exc.__traceback__)
    FrameType = type(_exc.__traceback__.tb_frame)
__all__ = ['NoneType', 'FunctionType', 'LambdaType', 'CodeType', 'CellType', 'MethodType', 'BuiltinFunctionType', 'BuiltinMethodType', 'WrapperDescriptorType', 'MethodWrapperType', 'MethodDescriptorType', 'ClassMethodDescriptorType', 'GetSetDescriptorType', 'MemberDescriptorType', 'GeneratorType', 'CoroutineType', 'AsyncGeneratorType', 'FrameType', 'TracebackType', 'EllipsisType', 'NotImplementedType', 'UnionType', 'ModuleType', 'MappingProxyType', 'SimpleNamespace', 'GenericAlias']
