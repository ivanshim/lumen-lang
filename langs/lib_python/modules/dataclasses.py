# From CPython 3.14, Lib/dataclasses.py, written out in Python: the part
# pprint needs -- field options, __repr__/__init__/__eq__ generation,
# is_dataclass, fields and make_dataclass -- carried here with the record
# base this library already uses. Generated __repr__ is the base record's
# own, so a class that defines __repr__ itself is left with its own.
class _Missing:
    pass

MISSING = _Missing()

class field:
    def __init__(self, default=MISSING, default_factory=MISSING, init=True,
                 repr=True, hash=None, compare=True, metadata=None):
        if default is not MISSING and default_factory is not MISSING:
            raise 'ValueError: cannot specify both default and default_factory'
        self.default = default
        self.default_factory = default_factory
        self.init = init
        self.repr = repr
        self.hash = hash
        self.compare = compare
        self.metadata = {} if metadata is None else metadata
        self.name = None
        self.type = None

def _default_repr(self):
    return object.__repr__(self)


class _Params:
    def __init__(self, init, repr, eq, frozen):
        self.init = init
        self.repr = repr
        self.eq = eq
        self.frozen = frozen

_repr_running = set()

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
            spec = self._defaults[i]
            if not spec.init:
                continue
            if i < len(args):
                if name in keywords:
                    raise 'TypeError: duplicate dataclass argument'
                value = args[i]
            elif name in keywords:
                value = keywords[name]
            else:
                if spec.default_factory is not MISSING:
                    value = spec.default_factory()
                elif spec.default is not MISSING:
                    value = spec.default
                else:
                    raise 'TypeError: missing dataclass argument'
            setattr(self, name, value)

    def __repr__(self):
        key = id(self)
        if key in _repr_running:
            return '...'
        _repr_running.add(key)
        try:
            parts = []
            for name, spec in zip(self._fields, self._defaults):
                if not spec.repr:
                    continue
                value = getattr(self, name)
                if isinstance(value, _Record):
                    value = value.__repr__()
                else:
                    value = '%r' % (value,)
                parts.append(name + '=' + value)
            return self._record_name + '(' + ', '.join(parts) + ')'
        finally:
            _repr_running.discard(key)

    def __eq__(self, other):
        if not isinstance(other, _Record):
            return False
        if self._record_token is not other._record_token:
            return False
        for name in self._fields:
            if getattr(self, name) != getattr(other, name):
                return False
        return True

def _lay_out(cls_name, names, annotations, defaults, init, repr, eq, frozen,
             carried):
    fields = {}
    for name, spec in zip(names, defaults):
        fields[name] = spec
    members = {
        '_fields': names,
        '_defaults': defaults,
        '_record_name': cls_name,
        '_record_token': _Missing(),
        '__name__': cls_name,
        '__qualname__': cls_name,
        '__annotations__': annotations,
        '__dataclass_params__': _Params(init, repr, eq, frozen),
        '__dataclass_fields__': fields,
    }
    if not repr:
        members['__repr__'] = _default_repr
    for word, value in carried:
        members[word] = value
    return __derive_class(cls_name, _Record, members)

def _process_class(cls, init, repr, eq, frozen):
    if frozen:
        raise 'NotImplementedError: these dataclass options are not supported'
    if len(cls.__bases__) and cls.__bases__[0].__name__ != 'object':
        raise 'NotImplementedError: dataclass inheritance is not supported'
    annotations = dict(getattr(cls, '__annotations__', {}))
    names = list(annotations)
    defaults = []
    for name in names:
        value = getattr(cls, name, MISSING)
        spec = value if isinstance(value, field) else field(default=value)
        spec.name = name
        spec.type = annotations.get(name)
        defaults.append(spec)
    carried = []
    for word in __class_methods(cls):
        carried.append((word, getattr(cls, word)))
    return _lay_out(cls.__name__, names, annotations, defaults, init, repr,
                    eq, frozen, carried)

def dataclass(cls=None, *, init=True, repr=True, eq=True, order=False,
              unsafe_hash=False, frozen=False, match_args=True, kw_only=False,
              slots=False):
    def wrap(cls):
        return _process_class(cls, init, repr, eq, frozen)
    if cls is None:
        return wrap
    return wrap(cls)

def is_dataclass(obj):
    return hasattr(obj, '__dataclass_params__')

def fields(obj):
    cls = obj if isinstance(obj, type) else type(obj)
    table = getattr(cls, '__dataclass_fields__', None)
    if table is None:
        raise 'TypeError: not a dataclass'
    order = getattr(cls, '_fields', [])
    return [table[name] for name in order]

def make_dataclass(cls_name, fields, *, bases=(), namespace=None, init=True,
                   repr=True, eq=True, order=False, unsafe_hash=False,
                   frozen=False, match_args=True, kw_only=False, slots=False,
                   module=None, qualname=None, doc=None):
    names = []
    annotations = {}
    defaults = []
    for item in fields:
        if isinstance(item, str):
            name, spec = item, field()
        elif len(item) == 2:
            name, spec = item[0], field(default=item[1])
        else:
            name, spec = item[0], (item[2] if isinstance(item[2], field) else field(default=item[2]))
        names.append(name)
        annotations[name] = None
        spec.name = name
        defaults.append(spec)
    return _lay_out(cls_name, names, annotations, defaults, init, repr, eq,
                    frozen, [])

def asdict(obj):
    if not isinstance(obj, _Record):
        raise 'TypeError: asdict should be called on dataclass instances'
    result = {}
    for name in obj._fields:
        value = getattr(obj, name)
        result[name] = asdict(value) if isinstance(value, _Record) else __copy_value(value, True)
    return result
