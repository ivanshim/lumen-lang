# Fields are kept in declaration order; each factory is called for its object.
class _Missing:
    pass

MISSING = _Missing()

class FrozenInstanceError(AttributeError):
    pass

class field:
    def __init__(self, default=MISSING, default_factory=MISSING, **options):
        unknown = [name for name in options if name != 'kw_only']
        if len(unknown):
            raise 'NotImplementedError: these field options are not supported'
        if default is not MISSING and default_factory is not MISSING:
            raise 'ValueError: cannot specify both default and default_factory'
        self.default = default
        self.default_factory = default_factory
        self.kw_only = options.get('kw_only', MISSING)

# What a call of field() makes: the factory and the type are one here.
Field = field

class _Record:
    def __init__(self, *args, **keywords):
        names = self._fields
        kw_only = getattr(type(self), '_kw_only_fields', [])
        positional_names = [name for name in names if name not in kw_only]
        if len(args) > len(positional_names):
            raise 'TypeError: too many dataclass arguments'
        for name in list(keywords):
            if name not in names:
                raise 'TypeError: unexpected dataclass argument'
        given = {}
        for i in range(len(args)):
            given[positional_names[i]] = args[i]
        for name in keywords:
            if name in given:
                raise 'TypeError: duplicate dataclass argument'
            given[name] = keywords[name]
        for i in range(len(names)):
            name = names[i]
            if name in given:
                value = given[name]
            else:
                specification = self._defaults[i]
                if specification.default_factory is not MISSING:
                    value = specification.default_factory()
                elif specification.default is not MISSING:
                    value = specification.default
                else:
                    raise 'TypeError: missing dataclass argument'
            setattr(self, name, value)
        post = getattr(self, '__post_init__', None)
        if post is not None:
            post()
        self._frozen_sealed = True

    def __setattr__(self, name, value):
        if getattr(self, '_frozen_sealed', False) and name in self._fields:
            raise FrozenInstanceError("cannot assign to field '" + name + "'")
        object.__setattr__(self, name, value)

    def __delattr__(self, name):
        if getattr(self, '_frozen_sealed', False) and name in self._fields:
            raise FrozenInstanceError('cannot delete field ' + repr(name))
        object.__delattr__(self, name)

    def __replace__(self, /, **changes):
        for name in changes:
            if name not in self._fields:
                raise TypeError(type(self).__name__ + ".__init__() got an unexpected keyword argument '" + name + "'")
        values = {name: changes.get(name, getattr(self, name)) for name in self._fields}
        return type(self)(**values)

    def __repr__(self):
        result = self._record_name + '('
        for i in range(len(self._fields)):
            if i:
                result += ', '
            name = self._fields[i]
            value = getattr(self, name)
            if isinstance(value, _Record):
                value = value.__repr__()
            else:
                value = '%r' % (value,)
            result += name + '=' + value
        return result + ')'

    def __eq__(self, other):
        if not isinstance(other, _Record):
            return False
        if self._record_token is not other._record_token:
            return False
        for name in self._fields:
            if getattr(self, name) != getattr(other, name):
                return False
        return True

# Stub: a fresh record class is made. Frozen records, inheritance and
# custom methods are not provided.
def dataclass(cls=None, frozen=False, **options):
    if cls is None:
        return lambda c: dataclass(c, frozen=frozen, **options)
    unknown = [name for name in options if name != 'kw_only']
    if len(unknown):
        raise 'NotImplementedError: these dataclass options are not supported'
    kw_only_class = bool(options.get('kw_only', False))
    base = cls.__bases__[0] if len(cls.__bases__) else None
    base_fields = []
    base_defaults = []
    base_kw_only = []
    if base is not None and base.__name__ != 'object':
        if getattr(base, '_fields', None) is not None:
            # Building on a record, each of the two frozen or not.
            if getattr(base, '_frozen', False) and not frozen:
                raise 'TypeError: cannot inherit non-frozen dataclass from a frozen one'
            if not getattr(base, '_frozen', False) and frozen:
                raise 'TypeError: cannot inherit frozen dataclass from a non-frozen one'
            base_fields = list(base._fields)
            base_defaults = list(base._defaults)
            base_kw_only = list(getattr(base, '_kw_only_fields', []))
    names = list(getattr(cls, '__annotations__', {}))
    defaults = []
    kw_only = []
    optional = len(base_defaults) > 0 and any(
        s.default is not MISSING or s.default_factory is not MISSING for s in base_defaults)
    for name in names:
        value = getattr(cls, name, MISSING)
        specification = value if isinstance(value, field) else field(default=value)
        supplied = specification.default is not MISSING or specification.default_factory is not MISSING
        if optional and not supplied:
            raise 'TypeError: non-default argument follows default argument'
        optional = optional or supplied
        defaults.append(specification)
        if kw_only_class or specification.kw_only is True:
            kw_only.append(name)
    namespace = {'_fields': base_fields + names, '_defaults': base_defaults + defaults,
                 '_kw_only_fields': base_kw_only + kw_only, '_record_name': cls.__name__,
                 '_record_token': _Missing(), '__name__': cls.__name__,
                 '__annotations__': getattr(cls, '__annotations__', {}),
                 '__init__': _Record.__init__, '__setattr__': _Record.__setattr__,
                 '__delattr__': _Record.__delattr__, '_frozen': frozen}
    return __derive_class(cls.__name__, cls, namespace)

def asdict(obj):
    if not isinstance(obj, _Record):
        raise 'TypeError: asdict should be called on dataclass instances'
    result = {}
    for name in obj._fields:
        value = getattr(obj, name)
        result[name] = asdict(value) if isinstance(value, _Record) else __copy_value(value, True)
    return result
