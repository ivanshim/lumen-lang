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


# Export the runtime types used by class namespaces and sys.implementation.
# Their identity and native protocols are shared by every construction path.
MappingProxyType = type(type.__dict__)
SimpleNamespace = __namespace_type()


# What a kind is called. A builtin kind answers to __name__ but is not
# found by getattr, so the name is asked for directly.
def _kind_name(kind):
    try:
        name = str(kind.__qualname__)
        home = kind.__module__
    except BaseException:
        try:
            return str(kind.__name__)
        except BaseException:
            return repr(kind)
    if home in ('builtins', None):
        return name
    return str(home) + '.' + name


# Generic aliases use the runtime's type, including iterable unpacking,
# substitution, and identity shared with builtin subscriptions.
GenericAlias = type(list[int])

UnionType = type(int | str)
# What a built union (int | str) answers when it is subscripted, pickled
# or hashed; the runtime hands the union itself to each.
def _substituted(thing, item):
    params = thing.__parameters__
    if not params:
        raise TypeError(repr(thing) + ' is not a generic class')
    if not isinstance(item, tuple):
        item = (item,)
    if len(item) != len(params):
        few = 'few' if len(item) < len(params) else 'many'
        raise TypeError('Too %s arguments for %r; actual %d, expected %d' % (few, thing, len(item), len(params)))
    chosen = [(p, p.__typing_subst__(v) if hasattr(p, '__typing_subst__') else v) for p, v in zip(params, item)]
    new = []
    for arg in thing.__args__:
        found = [v for p, v in chosen if p is arg]
        if found:
            new.append(found[0])
            continue
        inner = getattr(arg, '__parameters__', ())
        if inner:
            new.append(arg[tuple([v for p in inner for q, v in chosen if q is p])])
        else:
            new.append(arg)
    return tuple(new)


def _union_getitem(union, item):
    import typing
    return typing.Union[_substituted(union, item)]


def _alias_getitem(alias, item):
    return type(alias)(alias.__origin__, _substituted(alias, item))


def _union_reduce(union):
    import operator, typing
    return (operator.getitem, (typing.Union, union.__args__))


def _hash_parts(members):
    hashable, unhashable = [], []
    for member in members:
        try:
            hash(member)
        except TypeError:
            unhashable.append(member)
        else:
            hashable.append(member)
    return hashable, unhashable


def _union_args(members):
    # The members of a union made from these, in order, with the repeats
    # taken out: members that hash are alike when they compare equal among
    # those hashing alike, the rest when they compare equal to one another.
    kept, unhashable, seen = [], [], {}
    for member in members:
        try:
            key = hash(member)
        except TypeError:
            for other in unhashable:
                if other is member or other == member:
                    break
            else:
                kept.append(member)
                unhashable.append(member)
            continue
        bucket = seen.setdefault(key, [])
        for other in bucket:
            if other is member or other == member:
                break
        else:
            bucket.append(member)
            kept.append(member)
    return tuple(kept), tuple(unhashable)


def _union_eq(union, other):
    if type(other) is not type(union):
        return NotImplemented
    if union is other:
        return True
    mine, bad_mine = _hash_parts(union.__args__)
    theirs, bad_theirs = _hash_parts(other.__args__)
    if len(mine) != len(theirs) or len(bad_mine) != len(bad_theirs):
        return False
    if frozenset(mine) != frozenset(theirs):
        return False
    for member in bad_mine:
        for candidate in bad_theirs:
            if member is candidate or member == candidate:
                break
        else:
            return False
    return True


def _union_new(cls, *args, **kwargs):
    raise TypeError("cannot create 'typing.Union' instances")


def _union_hash(union):
    return hash(frozenset(union.__args__))


# mappingproxy[int], as a kind is subscripted.
def _alias_of_class(cls, item):
    return GenericAlias(cls, item)


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


GenericAlias = type(list[int])

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
CapsuleType = __capsule_type()
__all__ = ['CapsuleType', 'NoneType', 'FunctionType', 'LambdaType', 'CodeType', 'CellType', 'MethodType', 'BuiltinFunctionType', 'BuiltinMethodType', 'WrapperDescriptorType', 'MethodWrapperType', 'MethodDescriptorType', 'ClassMethodDescriptorType', 'GetSetDescriptorType', 'MemberDescriptorType', 'GeneratorType', 'CoroutineType', 'AsyncGeneratorType', 'FrameType', 'TracebackType', 'EllipsisType', 'NotImplementedType', 'UnionType', 'ModuleType', 'MappingProxyType', 'SimpleNamespace', 'GenericAlias']
