# Stub: hints carry no checks at run time and keep no supplied parameters.
class _Hint:
    def __getitem__(self, parameters):
        return self

Any = _Hint()
Optional = _Hint()
Union = _Hint()
List = _Hint()
Dict = _Hint()
Tuple = _Hint()
Set = _Hint()
Callable = _Hint()

class Protocol:
    def __getitem__(cls, parameters):
        return cls

from _typing import TypeVar, ParamSpec, TypeVarTuple, TypeAliasType, Generic, NoDefault

def cast(typ, value):
    return value

TYPE_CHECKING = False

# Runtime numeric protocols use the same structural hook as abstract bases.
from abc import ABCMeta

class _NumericProtocol(metaclass=ABCMeta):
    @classmethod
    def __subclasshook__(cls, candidate):
        return getattr(candidate, cls._required_method, None) is not None

class SupportsInt(_NumericProtocol):
    _required_method = '__int__'

class SupportsFloat(_NumericProtocol):
    _required_method = '__float__'

class SupportsComplex(_NumericProtocol):
    _required_method = '__complex__'

class SupportsIndex(_NumericProtocol):
    _required_method = '__index__'

class SupportsBytes(_NumericProtocol):
    _required_method = '__bytes__'

class SupportsAbs(_NumericProtocol):
    _required_method = '__abs__'

class SupportsRound(_NumericProtocol):
    _required_method = '__round__'


# Resolve the runtime annotations available on ordinary functions and classes.
def get_type_hints(obj, globalns=None, localns=None, include_extras=False, *, format=1):
    if getattr(obj, '__no_type_check__', False):
        return {}
    if format not in (1, 2, 3, 4):
        raise ValueError(str(format) + ' is not a valid Format')
    if format == 4:
        raise ValueError('The STRING format is not supported by get_type_hints()')
    from annotationlib import get_annotations
    hints = get_annotations(obj)
    if globalns is None:
        globalns = getattr(obj, '__globals__', {})
    if localns is None:
        localns = globalns
    resolved = {}
    for name, value in hints.items():
        if isinstance(value, str):
            value = eval(value, globalns, localns)
        if value is None:
            value = type(None)
        resolved[name] = value
    return resolved
