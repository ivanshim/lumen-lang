# Members are made once, so aliases and value lookup retain identity.
# Stub: mixed-in types, flags and custom member constructors are not carried.
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
        return '<' + self.owner.__name__ + '.' + self.name + ': ' + ('%r' % (self.value,)) + '>'

    def __eq__(self, other):
        return self is other

class Enum:
    def __init_subclass__(cls, **options):
        attributes = vars(cls)
        members = []
        next_value = 1
        for name in list(attributes):
            if name[:1] != '_' and not callable(attributes[name]):
                value = attributes[name]
                if isinstance(value, auto):
                    value = next_value
                if type(value) == type(0) and (len(members) == 0 or value >= next_value):
                    next_value = value + 1
                member = None
                for held in members:
                    if held.value == value:
                        member = held
                if member is None:
                    member = _Member(cls, name, value)
                    members.append(member)
                setattr(cls, name, member)
        setattr(cls, '_members', members)

    def __class_call__(cls, value):
        for member in cls._members:
            if member is value or member.value == value:
                return member
        raise 'ValueError: value is not an enumeration member'

    def __class_iter__(cls):
        return cls._members

# Stub: integer arithmetic on members awaits numeric object methods.
class IntEnum(Enum):
    def __init_subclass__(cls, **options):
        raise 'NotImplementedError: integer enumeration arithmetic is not supported'
