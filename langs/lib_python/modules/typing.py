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

class Generic:
    def __getitem__(cls, parameters):
        return cls

class Protocol:
    def __getitem__(cls, parameters):
        return cls

class TypeVar:
    def __init__(self, name, *constraints, **options):
        self.__name__ = name

    def __getitem__(self, parameters):
        return self

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
