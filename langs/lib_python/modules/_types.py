# Bindings and constructors for CPython Modules/_typesmodule.c.
# PSF License.
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


class _MethodSample:
    def method(self): pass
MethodType = type(_MethodSample().method)
del _MethodSample


# ModuleType is the runtime's native module class, including its readonly
# namespace descriptor and subclass initialization protocol.
import sys
ModuleType = type(sys)


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
        return self._mapping.copy()

    def __eq__(self, other):
        if isinstance(other, MappingProxyType):
            return self.copy() == other.copy()
        return self.copy() == other

    # A reading stands for the mapping it reads when the union sign
    # reaches it, on either side. Either side that is itself a reading
    # is unwrapped first, and the ordinary union then runs on the
    # mappings themselves, so a mapping's own answer and the kind it
    # answers with are kept: a mapping whose union is its own returns
    # that answer through the reading as well. A reading cannot be
    # written through with the in-place sign.
    def __or__(self, other):
        right = other._mapping if isinstance(other, MappingProxyType) else other
        return self._mapping | right

    def __ror__(self, other):
        left = other._mapping if isinstance(other, MappingProxyType) else other
        return left | self._mapping

    def __ior__(self, other):
        raise TypeError("'|=' is not supported by mappingproxy; use '|' instead")

    def __repr__(self):
        return 'mappingproxy(' + repr(self._mapping) + ')'


# The reference names this kind mappingproxy, and types.MappingProxyType
# is that kind, so the name a type prints is the reference's.
MappingProxyType.__name__ = 'mappingproxy'


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


# A kind named with the kinds it was given, as list[int] is: the one class the
# runtime makes every such alias of.
GenericAlias = type(list[int])


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
UnionType = type(int | str)

__all__ = ['NoneType', 'FunctionType', 'LambdaType', 'CodeType', 'CellType', 'MethodType', 'BuiltinFunctionType', 'BuiltinMethodType', 'WrapperDescriptorType', 'MethodWrapperType', 'MethodDescriptorType', 'ClassMethodDescriptorType', 'GetSetDescriptorType', 'MemberDescriptorType', 'GeneratorType', 'CoroutineType', 'AsyncGeneratorType', 'FrameType', 'TracebackType', 'EllipsisType', 'NotImplementedType', 'UnionType', 'ModuleType', 'MappingProxyType', 'SimpleNamespace', 'GenericAlias']
