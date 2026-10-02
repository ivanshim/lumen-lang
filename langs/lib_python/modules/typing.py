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

# Adapted from CPython Lib/typing.py at v3.14.8 / 8e6e75d9102e; PSF License.
# Named tuple classes use the same tuple factory as their functional form.
class _NamedTupleMeta(type):
    def __call__(cls, typename, fields, /):
        from collections import namedtuple
        annotations = dict(fields)
        result = namedtuple(typename, list(annotations))
        result.__annotations__ = annotations
        return result

    def __new__(mcls, name, bases, namespace):
        from collections import namedtuple
        prototype = type.__new__(mcls, name, bases, namespace)
        annotations = getattr(prototype, '__annotations__', {})
        if name == 'NamedTuple' and not annotations:
            return prototype
        fields = list(annotations)
        defaults = []
        found_default = False
        for field in fields:
            if field in namespace:
                found_default = True
                defaults.append(namespace[field])
            elif found_default:
                raise TypeError("Non-default namedtuple field " + field + " cannot follow default fields")
        result = namedtuple(name, fields, defaults=defaults, module=namespace.get('__module__'))
        result.__annotations__ = annotations
        for member, value in namespace.items():
            if member not in fields and member not in ('__module__', '__annotations__', '__slots__'):
                setattr(result, member, value)
        return result

class NamedTuple(metaclass=_NamedTupleMeta):
    pass
