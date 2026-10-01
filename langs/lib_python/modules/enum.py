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
    def __init_subclass__(cls, **options):
        members = []
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
        cls._members = members

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

# The reference's global_enum files an enumeration's members under their
# own names in the module the enumeration was written in. Here the module
# is reached through the language's module cache; a module the cache has
# not kept (or cannot be written) simply keeps its members on the class.
def global_enum(cls, update_str=False):
    import sys
    home = sys.modules.get(cls.__module__)
    if home is not None:
        for member in cls._members:
            try:
                setattr(home, member.name, member)
            except AttributeError:
                pass
    return cls

class StrEnum(str, Enum):
    def __str__(self):
        return str(self.value)

    def __repr__(self):
        return '<' + self.__class__.__name__ + '.' + self.name + ': ' + repr(self.value) + '>'
