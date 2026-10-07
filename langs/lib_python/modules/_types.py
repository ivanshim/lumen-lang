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


# Export the runtime types used by class namespaces and sys.implementation.
# Their identity and native protocols are shared by every construction path.
MappingProxyType = type(type.__dict__)
SimpleNamespace = __namespace_type()


# What a kind is called. A builtin kind answers to __name__ but is not
# found by getattr, so the name is asked for directly.
def _kind_name(kind):
    try:
        return str(kind.__name__)
    except BaseException:
        return repr(kind)


# Generic aliases use the runtime's type, including iterable unpacking,
# substitution, and identity shared with builtin subscriptions.
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
UnionType = type(int | str)

__all__ = ['NoneType', 'FunctionType', 'LambdaType', 'CodeType', 'CellType', 'MethodType', 'BuiltinFunctionType', 'BuiltinMethodType', 'WrapperDescriptorType', 'MethodWrapperType', 'MethodDescriptorType', 'ClassMethodDescriptorType', 'GetSetDescriptorType', 'MemberDescriptorType', 'GeneratorType', 'CoroutineType', 'AsyncGeneratorType', 'FrameType', 'TracebackType', 'EllipsisType', 'NotImplementedType', 'UnionType', 'ModuleType', 'MappingProxyType', 'SimpleNamespace', 'GenericAlias']
