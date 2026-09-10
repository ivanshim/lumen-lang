# Fields are kept in declaration order; each factory is called for its object.
class _Missing:
    pass

MISSING = _Missing()

class field:
    def __init__(self, default=MISSING, default_factory=MISSING, **options):
        if len(options):
            raise 'NotImplementedError: these field options are not supported'
        if default is not MISSING and default_factory is not MISSING:
            raise 'ValueError: cannot specify both default and default_factory'
        self.default = default
        self.default_factory = default_factory

class _Record:
    def __init__(self, *args, **keywords):
        names = self._fields
        if len(args) > len(names):
            raise 'TypeError: too many dataclass arguments'
        for name in list(keywords):
            if name not in names:
                raise 'TypeError: unexpected dataclass argument'
        for i in range(len(names)):
            name = names[i]
            if i < len(args):
                if name in keywords:
                    raise 'TypeError: duplicate dataclass argument'
                value = args[i]
            elif name in keywords:
                value = keywords[name]
            else:
                specification = self._defaults[i]
                if specification.default_factory is not MISSING:
                    value = specification.default_factory()
                elif specification.default is not MISSING:
                    value = specification.default
                else:
                    raise 'TypeError: missing dataclass argument'
            setattr(self, name, value)

    def __repr__(self):
        result = self._record_name + '('
        for i in range(len(self._fields)):
            if i:
                result += ', '
            name = self._fields[i]
            value = getattr(self, name)
            if type(value) == type(''):
                value = "'" + value + "'"
            else:
                value = str(value)
            result += name + '=' + value
        return result + ')'

    def __eq__(self, other):
        if self.__class__ is not other.__class__:
            return False
        for name in self._fields:
            if getattr(self, name) != getattr(other, name):
                return False
        return True

# Stub: frozen records, inheritance and custom methods are not provided.
def dataclass(cls=None, frozen=False, **options):
    if frozen or len(options):
        raise 'NotImplementedError: these dataclass options are not supported'
    if cls is None:
        def decorate(cls):
            return dataclass(cls)
        return decorate
    if len(__class_methods(cls)):
        raise 'NotImplementedError: dataclasses with custom methods are not supported'
    names = list(getattr(cls, '__annotations__', {}))
    defaults = []
    optional = False
    for name in names:
        value = getattr(cls, name, MISSING)
        specification = value if isinstance(value, field) else field(default=value)
        supplied = specification.default is not MISSING or specification.default_factory is not MISSING
        if optional and not supplied:
            raise 'TypeError: non-default argument follows default argument'
        optional = optional or supplied
        defaults.append(specification)
    return __derive_class(cls.__name__, _Record, {'_fields': names, '_defaults': defaults, '_record_name': cls.__name__})

def asdict(obj):
    if not isinstance(obj, _Record):
        raise 'TypeError: asdict should be called on dataclass instances'
    result = {}
    for name in obj._fields:
        value = getattr(obj, name)
        result[name] = asdict(value) if isinstance(value, _Record) else __copy_value(value, True)
    return result
