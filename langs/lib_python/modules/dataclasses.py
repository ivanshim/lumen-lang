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

# The exact classes a frozen decoration made. CPython's
# _frozen_get_del_attr closes over the decorated class itself: on that
# class every attribute write is refused, while a subclass that was not
# itself decorated frozen keeps the inherited fields frozen but may
# grow attributes of its own.
_frozen_exact = []

def _is_frozen_exact(cls):
    for known in _frozen_exact:
        if known is cls:
            return True
    return False

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
            if getattr(type(self), '_frozen', False):
                # A frozen record's initializer writes past the frozen
                # setters, the way CPython's generated __init__ calls
                # object.__setattr__ for each field.
                object.__setattr__(self, name, value)
            else:
                setattr(self, name, value)
        post = getattr(self, '__post_init__', None)
        if post is not None:
            post()

    def __setattr__(self, name, value):
        frozen_fields = getattr(type(self), '_frozen_fields', None)
        if frozen_fields is not None and (_is_frozen_exact(type(self)) or name in frozen_fields):
            raise FrozenInstanceError('cannot assign to field ' + repr(name))
        object.__setattr__(self, name, value)

    def __delattr__(self, name):
        frozen_fields = getattr(type(self), '_frozen_fields', None)
        if frozen_fields is not None and (_is_frozen_exact(type(self)) or name in frozen_fields):
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
        if getattr(type(other), '_record_token', None) is None:
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
    annotations = getattr(cls, '__annotations__', {})
    names = []
    for name in annotations:
        annotation = annotations[name]
        if isinstance(annotation, str) and (annotation == 'ClassVar' or annotation.startswith('ClassVar[') or annotation == 'typing.ClassVar' or annotation.startswith('typing.ClassVar[')):
            continue
        names.append(name)
    # A field an ancestor declared keeps the place it was first
    # declared at; declaring the name again puts the new specification
    # at that old place, the way CPython's _process_class overrides.
    merged = {}
    order = []
    for i in range(len(base_fields)):
        merged[base_fields[i]] = base_defaults[i]
        order.append(base_fields[i])
    kw_only = list(base_kw_only)
    # The default-ordering rule watches the fields taken positionally
    # alone; keyword-only fields are free of it, ordered or not.
    optional = False
    last_defaulted = None
    for i in range(len(base_fields)):
        if base_fields[i] in base_kw_only:
            continue
        s = base_defaults[i]
        if s.default is not MISSING or s.default_factory is not MISSING:
            optional = True
            last_defaulted = base_fields[i]
    for name in names:
        value = getattr(cls, name, MISSING)
        specification = value if isinstance(value, field) else field(default=value)
        supplied = specification.default is not MISSING or specification.default_factory is not MISSING
        is_kw_only = kw_only_class or specification.kw_only is True
        if not is_kw_only:
            if optional and not supplied:
                raise 'TypeError: non-default argument ' + repr(name) + ' follows default argument ' + repr(last_defaulted)
            if supplied:
                optional = True
                last_defaulted = name
        specification.name = name
        if name in merged:
            merged[name] = specification
            if name in kw_only:
                kw_only.remove(name)
        else:
            order.append(name)
            merged[name] = specification
        if is_kw_only:
            kw_only.append(name)
    defaults = [merged[name] for name in order]
    dataclass_fields = {}
    for name in order:
        dataclass_fields[name] = merged[name]
    namespace = {'_fields': order, '_defaults': defaults,
                 '_kw_only_fields': kw_only, '_record_name': cls.__name__,
                 '_record_token': _Missing(), '__name__': cls.__name__,
                 '__annotations__': getattr(cls, '__annotations__', {}),
                 '__dataclass_fields__': dataclass_fields,
                 '__init__': _Record.__init__, '__setattr__': _Record.__setattr__,
                 '__delattr__': _Record.__delattr__, '__repr__': _Record.__repr__,
                 '__eq__': _Record.__eq__, '__replace__': _Record.__replace__, '_frozen': frozen}
    if frozen:
        namespace['_frozen_fields'] = order
    made = __derive_class(cls.__name__, cls, namespace)
    if frozen:
        _frozen_exact.append(made)
    return made

def fields(class_or_instance):
    # The field specifications in declaration order, as CPython's
    # fields() answers them; anything that is no record is refused.
    cls = class_or_instance if isinstance(class_or_instance, type) else type(class_or_instance)
    kept = getattr(cls, '_fields', None)
    if kept is None:
        raise 'TypeError: must be called with a dataclass type or instance'
    held = getattr(cls, '__dataclass_fields__', {})
    return tuple(held[name] for name in kept)


def asdict(obj):
    if getattr(type(obj), '_record_token', None) is None:
        raise 'TypeError: asdict should be called on dataclass instances'
    result = {}
    for name in obj._fields:
        value = getattr(obj, name)
        result[name] = asdict(value) if getattr(type(value), '_record_token', None) is not None else __copy_value(value, True)
    return result
