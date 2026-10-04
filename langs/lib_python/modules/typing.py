# Stub: hints carry no checks at run time and keep no supplied parameters.
# The container hints answer isinstance/issubclass through the class they
# stand for and join with `|` into a hint of their own. A parameterized
# alias, a special form that runs no check, and a special form used as a
# union all answer as the reference answers them: the first is refused
# outright, the second is refused by name, the third is left to the
# kernel as any other operand of `|`.
_hint_cache = {}


class _Hint:
    def __init__(self, name="", target=None, members=None, subscripted=False):
        self._name = name
        self._target = target
        self._members = members
        self._subscripted = subscripted

    def __getitem__(self, parameters):
        if self._name == "Optional":
            return _union_of((parameters, None))
        if self._name == "Union":
            return _union_of(parameters)
        # The reference keeps one alias for one origin and one set of
        # arguments, so `typing.Tuple[int, int]` twice is the one hint
        # and annotations built apart compare as the same.
        key = (self._name, parameters)
        try:
            found = _hint_cache.get(key)
        except TypeError:
            found = None
        if found is not None:
            return found
        made = _Hint(self._name, None, None, True)
        try:
            _hint_cache[key] = made
        except TypeError:
            pass
        return made

    def __or__(self, other):
        return _union_of((self, other))

    def __ror__(self, other):
        return _union_of((other, self))

    def _refuse(self, asked):
        if self._subscripted:
            raise TypeError("Subscripted generics cannot be used with class and instance checks")
        if self._name == "Optional":
            raise TypeError("typing.Optional cannot be used with " + asked + "()")

    def __instancecheck__(self, instance):
        self._refuse("isinstance")
        if self._name == "Any":
            raise TypeError("typing.Any cannot be used with isinstance()")
        if self._target is not None:
            return isinstance(instance, self._target)
        if self._members is not None:
            return any(isinstance(instance, member) for member in self._members)
        return False

    def __subclasscheck__(self, cls):
        self._refuse("issubclass")
        if self._name == "Any":
            return False
        if cls is self:
            return True
        if self._target is not None:
            return issubclass(cls, self._target)
        if self._members is not None:
            return any(issubclass(cls, member) for member in self._members)
        return False


def _union_of(parameters):
    members = []
    for member in _flatten(parameters):
        if member is None:
            member = type(None)
        if isinstance(member, _Hint) and member._members is not None:
            additions = member._members
        else:
            additions = [member]
        for addition in additions:
            if addition not in members:
                members.append(addition)
    if len(members) == 1:
        return members[0]
    return _Hint("Union", None, members)


def _flatten(parameters):
    if not isinstance(parameters, tuple):
        return [parameters]
    gathered = []
    for parameter in parameters:
        gathered.extend(_flatten(parameter))
    return gathered


Any = _Hint("Any")
Optional = _Hint("Optional")
Union = _Hint("Union")
List = _Hint("List", list)
Dict = _Hint("Dict", dict)
Tuple = _Hint("Tuple", tuple)
Set = _Hint("Set", set)
Callable = _Hint("Callable")

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
