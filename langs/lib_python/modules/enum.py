# Enumeration support, with flag algorithms adapted from CPython 3b564385e4c9 Lib/enum.py; PSF License.
# Enumeration members retain identity, including aliases and value lookup.
class auto:
    pass

class _Member:
    def __init__(self, owner, name, value):
        self.owner = owner
        self.name = name
        self.value = value

    def __str__(self):
        return self.owner.__name__ + '.' + self.name

    def __repr__(self):
        return '<' + self.owner.__name__ + '.' + self.name + ': ' + repr(self.value) + '>'

    def __eq__(self, other):
        return self is other

class Enum:
    _flag_type_ = False
    def __init_subclass__(cls, **options):
        if getattr(cls, '_flag_type_', False):
            _flag_members(cls, options.get('boundary', getattr(cls, '_boundary_', KEEP)))
            return
        members = []
        member_map = {}
        next_value = 1
        for name in list(vars(cls)):
            if name[:1] == '_' or callable(getattr(cls, name)):
                continue
            value = getattr(cls, name)
            if isinstance(value, auto):
                value = name.lower() if issubclass(cls, str) else next_value
            if type(value) == int and value >= next_value:
                next_value = value + 1
            member = None
            for held in members:
                if held.value == value:
                    member = held
                    break
            if member is None:
                if issubclass(cls, int):
                    member = int.__new__(cls, value)
                    member.name = name
                    member.value = value
                elif issubclass(cls, str):
                    member = str.__new__(cls, value)
                    member.name = name
                    member.value = value
                elif issubclass(cls, float):
                    member = float.__new__(cls, value)
                    member.name = name
                    member.value = value
                else:
                    member = _Member(cls, name, value)
                members.append(member)
            setattr(cls, name, member)
            member_map[name] = member
        cls._members = members
        cls._member_map_ = member_map
        cls._member_names_ = [member.name for member in members]
        from types import MappingProxyType
        cls.__members__ = MappingProxyType(member_map)

    def __class_call__(cls, value):
        for member in cls._members:
            if member is value or member.value == value:
                return member
        raise 'ValueError: value is not an enumeration member'

    def __class_iter__(cls):
        return cls._members

    def __repr__(self):
        return '<' + self.__class__.__name__ + '.' + self.name + ': ' + repr(self.value) + '>'

class IntEnum(int, Enum):
    def __neg__(self):
        return 0 - self.value

    def __pos__(self):
        return self.value

    def __abs__(self):
        return abs(self.value)

    def __invert__(self):
        return ~self.value

    def __str__(self):
        return str(self.value)

    def __repr__(self):
        return '<' + self.__class__.__name__ + '.' + self.name + ': ' + repr(self.value) + '>'

class StrEnum(str, Enum):
    def __str__(self):
        return str(self.value)

    def __repr__(self):
        return '<' + self.__class__.__name__ + '.' + self.name + ': ' + repr(self.value) + '>'


class ReprEnum(Enum):
    pass

class FlagBoundary(StrEnum):
    STRICT = 'strict'
    CONFORM = 'conform'
    EJECT = 'eject'
    KEEP = 'keep'

STRICT = FlagBoundary.STRICT
CONFORM = FlagBoundary.CONFORM
EJECT = FlagBoundary.EJECT
KEEP = FlagBoundary.KEEP

def _is_single_bit(value):
    return value > 0 and value & (value - 1) == 0

def _flag_members(cls, boundary):
    member_map = {}
    value_map = {}
    names = []
    members = []
    mask = singles = 0
    last = 0
    for name, value in list(vars(cls).items()):
        if name.startswith('_') or callable(value) or type(value).__name__ in ('property', 'classmethod', 'staticmethod'):
            continue
        if isinstance(value, auto):
            value = 1 << last.bit_length()
        if not isinstance(value, int):
            raise TypeError('flag values must be integers')
        last = max(last, value)
        member = value_map.get(value)
        if member is None:
            member = int.__new__(cls, value) if issubclass(cls, int) else object.__new__(cls)
            member._name_ = name
            member._value_ = value
            member._sort_order_ = len(names)
            value_map[value] = member
            if _is_single_bit(value):
                names.append(name)
                members.append(member)
                singles |= value
        member_map[name] = member
        setattr(cls, name, member)
        mask |= value
    cls._member_map_ = member_map
    cls._value2member_map_ = value_map
    cls._member_names_ = names
    cls._members = members
    cls._flag_mask_ = mask
    cls._singles_mask_ = singles
    cls._all_bits_ = (1 << mask.bit_length()) - 1
    cls._boundary_ = boundary
    from types import MappingProxyType
    cls.__members__ = MappingProxyType(member_map)

class Flag(Enum):
    _flag_type_ = True
    _boundary_ = STRICT
    _numeric_repr_ = repr

    @property
    def name(self):
        return self._name_
    @property
    def value(self):
        return self._value_

    def __class_call__(cls, value):
        try:
            member = cls._value2member_map_.get(value)
        except TypeError:
            member = None
        if member is not None:
            return member
        return cls._missing_(value)

    @classmethod
    def __class_getitem__(cls, name):
        return cls._member_map_[name]

    @classmethod
    def _missing_(cls, value):
        if not isinstance(value, int):
            raise ValueError('%r is not a valid %s' % (value, cls.__name__))
        original = value
        mask = cls._flag_mask_
        all_bits = cls._all_bits_
        if not ~all_bits <= value <= all_bits or value & (all_bits ^ mask):
            if cls._boundary_ is CONFORM:
                value &= mask
            elif cls._boundary_ is EJECT:
                return value
            elif cls._boundary_ is KEEP:
                if value < 0:
                    value += max(all_bits + 1, 1 << value.bit_length())
            elif cls._boundary_ is STRICT:
                width = max(value.bit_length(), mask.bit_length())
                raise ValueError('%r invalid value %r\n    given %s\n  allowed %s' %
                                 (cls, value, _binary(value, width), _binary(mask, width)))
            else:
                raise ValueError('%r unknown flag boundary %r' % (cls, cls._boundary_))
        if value < 0:
            value = all_bits + 1 + value if cls._boundary_ in (KEEP, EJECT) else cls._singles_mask_ & value
        present = [m for m in cls._members if m.value & value == m.value]
        combined = 0
        for member in present:
            combined |= member.value
        aliases = value & ~cls._singles_mask_
        if aliases:
            for member in cls._member_map_.values():
                if member not in present and member.value and member.value & value == member.value:
                    present.append(member)
                    combined |= member.value
        member = int.__new__(cls, value) if issubclass(cls, int) else object.__new__(cls)
        member._value_ = value
        member._name_ = '|'.join(m.name for m in present) if combined else None
        unknown = value ^ combined
        if combined and unknown:
            member._name_ += '|' + cls._numeric_repr_(unknown)
        member = cls._value2member_map_.setdefault(value, member)
        if original < 0:
            cls._value2member_map_[original] = member
        return member

    def __iter__(self):
        return iter([m for m in self.__class__._members if self.value & m.value == m.value])
    def __len__(self):
        return self.value.bit_count()
    def __bool__(self):
        return bool(self.value)
    def __contains__(self, other):
        if not isinstance(other, self.__class__):
            raise TypeError("unsupported operand type(s) for 'in': %r and %r" %
                            (type(other).__name__, self.__class__.__name__))
        return other.value & self.value == other.value
    def __repr__(self):
        name = self._name_
        return '<%s%s: %s>' % (self.__class__.__name__, '.' + name if name is not None else '', self._numeric_repr_(self._value_))
    def __str__(self):
        return '%s.%s' % (self.__class__.__name__, self._name_) if self._name_ is not None else '%s(%s)' % (self.__class__.__name__, self.value)
    def _other_value(self, other):
        if isinstance(other, self.__class__):
            return other.value
        if isinstance(self, int) and isinstance(other, int):
            return int(other)
        return NotImplemented
    def __or__(self, other):
        value = self._other_value(other)
        return NotImplemented if value is NotImplemented else self.__class__(self.value | value)
    __ror__ = __or__
    def __and__(self, other):
        value = self._other_value(other)
        return NotImplemented if value is NotImplemented else self.__class__(self.value & value)
    __rand__ = __and__
    def __xor__(self, other):
        value = self._other_value(other)
        return NotImplemented if value is NotImplemented else self.__class__(self.value ^ value)
    __rxor__ = __xor__
    def __invert__(self):
        value = ~self.value if self._boundary_ in (KEEP, EJECT) else self._singles_mask_ & ~self.value
        return self.__class__(value)
    def __reduce_ex__(self, protocol):
        return self.__class__, (self.value,)

class IntFlag(int, Flag, ReprEnum):
    _boundary_ = KEEP
    __repr__ = Flag.__repr__
    __or__ = Flag.__or__
    __ror__ = Flag.__ror__
    __and__ = Flag.__and__
    __rand__ = Flag.__rand__
    __xor__ = Flag.__xor__
    __rxor__ = Flag.__rxor__
    __invert__ = Flag.__invert__
    def __str__(self):
        return str(self.value)

def _binary(value, width):
    if value < 0:
        return '0b1 ' + format((1 << width) + value, '0%db' % width)
    return '0b0 ' + format(value, '0%db' % width)

def _simple_enum(etype=Enum, *, boundary=None, use_args=None):
    def convert(cls):
        namespace = dict(vars(cls))
        namespace.pop('__dict__', None)
        namespace.pop('__weakref__', None)
        namespace['__module__'] = cls.__module__
        if boundary is not None:
            namespace['_boundary_'] = boundary
        return type(cls.__name__, (etype,), namespace)
    return convert

import sys

def global_enum_repr(self):
    """
    use module.enum_name instead of class.enum_name

    the module is the last module in case of a multi-module name
    """
    module = self.__class__.__module__.split('.')[-1]
    return '%s.%s' % (module, self._name_)

def global_flag_repr(self):
    """
    use module.flag_name instead of class.flag_name

    the module is the last module in case of a multi-module name
    """
    module = self.__class__.__module__.split('.')[-1]
    cls_name = self.__class__.__name__
    if self._name_ is None:
        return "%s.%s(%r)" % (module, cls_name, self._value_)
    if _is_single_bit(self._value_):
        return '%s.%s' % (module, self._name_)
    if self._boundary_ is not FlagBoundary.KEEP:
        return '|'.join(['%s.%s' % (module, name) for name in self.name.split('|')])
    else:
        name = []
        for n in self._name_.split('|'):
            if n[0].isdigit():
                name.append(n)
            else:
                name.append('%s.%s' % (module, n))
        return '|'.join(name)

def global_str(self):
    """
    use enum_name instead of class.enum_name
    """
    if self._name_ is None:
        cls_name = self.__class__.__name__
        return "%s(%r)" % (cls_name, self._value_)
    else:
        return self._name_

def global_enum(cls, update_str=False):
    """
    decorator that makes the repr() of an enum member reference its module
    instead of its class; also exports all members to the enum's module's
    global namespace
    """
    if issubclass(cls, Flag):
        cls.__repr__ = global_flag_repr
    else:
        cls.__repr__ = global_enum_repr
    if not issubclass(cls, ReprEnum) or update_str:
        cls.__str__ = global_str
    sys.modules[cls.__module__].__dict__.update(cls.__members__)
    return cls
