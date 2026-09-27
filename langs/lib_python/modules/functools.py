# Wrapping keeps the function itself; metadata copying is a stub.
def _identity(function):
    return function

def wraps(wrapped, assigned=None, updated=None):
    return _identity

def reduce(function, sequence, *initial):
    seen = len(initial) != 0
    value = None
    if seen:
        value = initial[0]
    for item in sequence:
        if seen:
            value = function(value, item)
        else:
            value = item
            seen = True
    if not seen:
        raise 'TypeError: reduce() of empty iterable with no initial value'
    return value

class _Partial:
    def __init__(self, function, args, keywords):
        self.function = function
        self.args = args
        self.keywords = keywords

    def call(self, *args, **keywords):
        return self.function(*self.args, *args, **{**self.keywords, **keywords})

def partial(function, *args, **keywords):
    return _Partial(function, args, keywords).call

# Descriptor and dispatch names are present even where the object
# protocol cannot yet carry their behaviour.
class _CacheInfo:
    def __init__(self, hits, misses, maxsize, currsize):
        self.hits = hits
        self.misses = misses
        self.maxsize = maxsize
        self.currsize = currsize

class _Cache:
    def __init__(self, function, maxsize, typed):
        self.__wrapped__ = function
        self.maxsize = maxsize
        self.typed = typed
        self.entries = []
        self.hits = 0
        self.misses = 0

    def __call__(self, *args, **kwargs):
        if len(kwargs) != 0:
            raise 'NotImplementedError: cache keyword keys are not supported'
        for arg in args:
            if type(arg) == type([]) or isinstance(arg, dict):
                raise 'TypeError: unhashable cache argument'
        for i in range(len(self.entries)):
            entry = self.entries[i]
            equal = len(entry[0]) == len(args)
            if equal:
                for j in range(len(args)):
                    if entry[0][j] != args[j] or (self.typed and type(entry[0][j]) != type(args[j])):
                        equal = False
            if equal:
                self.hits += 1
                self.entries = [*self.entries[:i], *self.entries[i + 1:], entry]
                return entry[1]
        self.misses += 1
        value = self.__wrapped__(*args)
        if self.maxsize != 0:
            self.entries = [*self.entries, [list(args), value]]
            if self.maxsize is not None and len(self.entries) > self.maxsize:
                self.entries = self.entries[1:]
        return value

    def cache_info(self):
        return _CacheInfo(self.hits, self.misses, self.maxsize, len(self.entries))

    def cache_clear(self):
        self.entries = []
        self.hits = 0
        self.misses = 0

class _CacheDecorator:
    def __init__(self, maxsize, typed):
        self.maxsize = maxsize
        self.typed = typed

    def __call__(self, function):
        return _Cache(function, self.maxsize, self.typed)

def lru_cache(maxsize=128, typed=False):
    if maxsize is None or isinstance(maxsize, type(1)) or isinstance(maxsize, type(True)):
        if maxsize is not None:
            maxsize = int(maxsize)
            if maxsize < 0:
                maxsize = 0
        return _CacheDecorator(maxsize, typed)
    if isinstance(maxsize, type('')) or isinstance(maxsize, type(0.0)):
        raise 'TypeError: first argument must be an integer, a callable, or None'
    return _Cache(maxsize, 128, typed)

def cache(user_function):
    return _Cache(user_function, None, False)

class cached_property:
    def __init__(self, func):
        raise 'NotImplementedError: cached_property needs descriptor lookup'

def singledispatch(func):
    raise 'NotImplementedError: singledispatch needs callable objects with attributes'

def cmp_to_key(mycmp):
    raise 'NotImplementedError: cmp_to_key needs object ordering methods'

def _gt_from_lt(self, other):
    result = type(self).__lt__(self, other)
    if result is NotImplemented:
        return NotImplemented
    return not result and self != other

def _le_from_lt(self, other):
    result = type(self).__lt__(self, other)
    if result is NotImplemented:
        return NotImplemented
    return result or self == other

def _ge_from_lt(self, other):
    result = type(self).__lt__(self, other)
    if result is NotImplemented:
        return NotImplemented
    return not result

def _ge_from_le(self, other):
    result = type(self).__le__(self, other)
    if result is NotImplemented:
        return NotImplemented
    return not result or self == other

def _lt_from_le(self, other):
    result = type(self).__le__(self, other)
    if result is NotImplemented:
        return NotImplemented
    return result and self != other

def _gt_from_le(self, other):
    result = type(self).__le__(self, other)
    if result is NotImplemented:
        return NotImplemented
    return not result

def _lt_from_gt(self, other):
    result = type(self).__gt__(self, other)
    if result is NotImplemented:
        return NotImplemented
    return not result and self != other

def _ge_from_gt(self, other):
    result = type(self).__gt__(self, other)
    if result is NotImplemented:
        return NotImplemented
    return result or self == other

def _le_from_gt(self, other):
    result = type(self).__gt__(self, other)
    if result is NotImplemented:
        return NotImplemented
    return not result

def _le_from_ge(self, other):
    result = type(self).__ge__(self, other)
    if result is NotImplemented:
        return NotImplemented
    return not result or self == other

def _gt_from_ge(self, other):
    result = type(self).__ge__(self, other)
    if result is NotImplemented:
        return NotImplemented
    return result and self != other

def _lt_from_ge(self, other):
    result = type(self).__ge__(self, other)
    if result is NotImplemented:
        return NotImplemented
    return not result

def total_ordering(cls):
    roots = []
    for name in ('__lt__', '__le__', '__gt__', '__ge__'):
        for base in cls.__mro__[:-1]:
            if name in base.__dict__:
                if name not in roots:
                    roots.append(name)
    if not roots:
        raise ValueError('must define at least one ordering operation: < > <= >=')
    root = max(roots)
    if root == '__lt__':
        methods = [('__gt__', _gt_from_lt), ('__le__', _le_from_lt), ('__ge__', _ge_from_lt)]
    if root == '__le__':
        methods = [('__ge__', _ge_from_le), ('__lt__', _lt_from_le), ('__gt__', _gt_from_le)]
    if root == '__gt__':
        methods = [('__lt__', _lt_from_gt), ('__ge__', _ge_from_gt), ('__le__', _le_from_gt)]
    if root == '__ge__':
        methods = [('__le__', _le_from_ge), ('__gt__', _gt_from_ge), ('__lt__', _lt_from_ge)]
    for name, method in methods:
        if name not in roots:
            setattr(cls, name, method)
    return cls
