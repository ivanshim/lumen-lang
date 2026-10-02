# Conservative dataclass implementation: unsupported options are refused.
class _Missing:
    pass
MISSING = _Missing()

class FrozenInstanceError(AttributeError):
    pass

class Field:
    def __init__(self, default=MISSING, default_factory=MISSING, init=True, repr=True, compare=True, hash=None, kw_only=False, metadata=None):
        self.default = default
        self.default_factory = default_factory
        self.init = init
        self.repr = repr
        self.compare = compare
        self.hash = hash
        self.kw_only = kw_only
        self.metadata = {}.keys().mapping if metadata is None else metadata
        self.name = None
        self.type = None
    @classmethod
    def __class_getitem__(cls, item):
        return cls

def field(*, default=MISSING, default_factory=MISSING, init=True, repr=True, compare=True, hash=None, kw_only=MISSING, metadata=None, **options):
    if options or hash is not None or metadata is not None:
        raise NotImplementedError('these field options are not supported')
    if default is not MISSING and default_factory is not MISSING:
        raise ValueError('cannot specify both default and default_factory')
    return Field(default, default_factory, init, repr, compare, hash, kw_only)

class _Params:
    def __init__(self, init, repr, eq, frozen):
        self.init = init
        self.repr = repr
        self.eq = eq
        self.frozen = frozen

_repr_running = []

def __create_fn__(cls, specifications, params):
    original_hash = cls.__dict__.get('__hash__', MISSING)
    explicit_hash = original_hash is not MISSING and not (original_hash is None and '__eq__' in cls.__dict__)
    def __init__(self, *args, **kwargs):
        positional = [f for f in specifications if f.init and not f.kw_only]
        accepted = [f.name for f in specifications if f.init]
        if len(args) > len(positional):
            raise TypeError('too many positional arguments')
        for name in kwargs:
            if name not in accepted:
                raise TypeError('unexpected keyword argument ' + repr(name))
        bound = dict(kwargs)
        for f, value in zip(positional, args):
            if f.name in bound:
                raise TypeError('multiple values for argument ' + repr(f.name))
            bound[f.name] = value
        for f in specifications:
            if f.init and f.name in bound:
                value = bound[f.name]
            elif f.default_factory is not MISSING:
                value = f.default_factory()
            elif f.default is not MISSING:
                if not f.init:
                    continue
                value = f.default
            elif not f.init:
                continue
            else:
                raise TypeError('missing required argument ' + repr(f.name))
            object.__setattr__(self, f.name, value)
        if hasattr(self, '__post_init__'):
            self.__post_init__()
    def __repr__(self):
        return type(self).__qualname__ + '(' + ', '.join(f.name + '=' + repr(getattr(self, f.name)) for f in specifications if f.repr) + ')'
    def wrapped_repr(self):
        key = id(self)
        if key in _repr_running:
            return '...'
        _repr_running.append(key)
        try:
            return __repr__(self)
        finally:
            _repr_running.remove(key)
    wrapped_repr.__wrapped__ = __repr__
    def __eq__(self, other):
        if type(other) is not type(self):
            return NotImplemented
        return tuple(getattr(self, f.name) for f in specifications if f.compare) == tuple(getattr(other, f.name) for f in specifications if f.compare)
    def __hash__(self):
        return hash(tuple(getattr(self, f.name) for f in specifications if f.compare))
    def __setattr__(self, name, value):
        if type(self) is cls or name in [f.name for f in specifications]:
            raise FrozenInstanceError('cannot assign to field ' + repr(name))
        super(cls, self).__setattr__(name, value)
    def __delattr__(self, name):
        if type(self) is cls or name in [f.name for f in specifications]:
            raise FrozenInstanceError('cannot delete field ' + repr(name))
        super(cls, self).__delattr__(name)
    if params.init and '__init__' not in cls.__dict__:
        setattr(cls, '__init__', __init__)
    if params.repr and '__repr__' not in cls.__dict__:
        setattr(cls, '__repr__', wrapped_repr)
    if params.eq and '__eq__' not in cls.__dict__:
        setattr(cls, '__eq__', __eq__)
    if params.frozen:
        if '__setattr__' in cls.__dict__ or '__delattr__' in cls.__dict__:
            raise TypeError('cannot overwrite frozen attribute methods')
        setattr(cls, '__setattr__', __setattr__)
        setattr(cls, '__delattr__', __delattr__)
    if not explicit_hash and params.eq:
        setattr(cls, '__hash__', __hash__ if params.frozen else None)

def dataclass(cls=None, *, init=True, repr=True, eq=True, frozen=False, kw_only=False, **options):
    if options:
        raise NotImplementedError('these dataclass options are not supported')
    def decorate(cls):
        specifications = {}
        for base in reversed(cls.__mro__[1:]):
            if '__dataclass_fields__' in base.__dict__:
                if bool(base.__dataclass_params__.frozen) != bool(frozen):
                    raise TypeError('cannot mix frozen and non-frozen dataclasses')
                specifications.update(base.__dataclass_fields__)
        annotations = getattr(cls, '__annotations__', {})
        for annotation in annotations.values():
            if isinstance(annotation, str) and annotation.split('[', 1)[0].strip().split('.')[-1] in ('ClassVar', 'InitVar'):
                raise NotImplementedError('ClassVar and InitVar annotations are not supported')
            if type(annotation).__name__ == '_Hint':
                raise NotImplementedError('erased typing annotations are not supported by dataclasses')
            if getattr(annotation, '__origin__', None) is not None:
                origin = annotation.__origin__
                if getattr(origin, '__name__', '') in ('ClassVar', 'InitVar'):
                    raise NotImplementedError('ClassVar and InitVar annotations are not supported')
        for name in annotations:
            value = getattr(cls, name, MISSING)
            f = value if isinstance(value, Field) else field(default=value)
            if f.default is not MISSING and (isinstance(f.default, (list, dict, set)) or getattr(type(f.default), '__hash__', object.__hash__) is None):
                raise ValueError('mutable default is not allowed: use default_factory')
            f.name = name
            f.type = annotations[name]
            if f.kw_only is MISSING:
                f.kw_only = kw_only
            specifications[name] = f
            if isinstance(value, Field):
                if f.default is MISSING:
                    delattr(cls, name)
                else:
                    setattr(cls, name, f.default)
        for name, value in cls.__dict__.items():
            if isinstance(value, Field) and name not in annotations:
                raise TypeError('field has no type annotation: ' + repr(name))
        optional = False
        for f in specifications.values():
            if f.init and not f.kw_only:
                supplied = f.default is not MISSING or f.default_factory is not MISSING
                if optional and not supplied:
                    raise TypeError('non-default argument follows default argument')
                optional = optional or supplied
        params = _Params(init, repr, eq, frozen)
        setattr(cls, '__dataclass_fields__', specifications)
        setattr(cls, '__dataclass_params__', params)
        setattr(cls, '__match_args__', tuple(f.name for f in specifications.values() if f.init and not f.kw_only))
        __create_fn__(cls, list(specifications.values()), params)
        return cls
    return decorate if cls is None else decorate(cls)

def is_dataclass(obj):
    cls = obj if isinstance(obj, type) else type(obj)
    return hasattr(cls, '__dataclass_fields__')

def fields(obj):
    if not is_dataclass(obj):
        raise TypeError('must be called with a dataclass type or instance')
    return tuple(obj.__dataclass_fields__.values())

def asdict(obj):
    if not is_dataclass(obj) or isinstance(obj, type):
        raise TypeError('asdict() should be called on dataclass instances')
    return {f.name: asdict(getattr(obj, f.name)) if is_dataclass(getattr(obj, f.name)) else __copy_value(getattr(obj, f.name), True) for f in fields(obj)}

def make_dataclass(cls_name, fields, *, bases=(), namespace=None, module=None, **options):
    from keyword import iskeyword
    attributes = {} if namespace is None else dict(namespace)
    annotations = {}
    for item in fields:
        if isinstance(item, str):
            name, annotation = item, 'typing.Any'
        elif len(item) == 2:
            name, annotation = item
        elif len(item) == 3:
            name, annotation, default = item
            attributes[name] = default
        else:
            raise TypeError('Invalid field: ' + repr(item))
        if not isinstance(name, str) or not name.isidentifier() or iskeyword(name) or name in annotations:
            raise TypeError('Invalid or duplicate field name: ' + repr(name))
        annotations[name] = annotation
    attributes['__annotations__'] = annotations
    attributes['__module__'] = __frame_module(1) if module is None else module
    cls = type(cls_name, bases, attributes)
    return dataclass(cls, **options)
