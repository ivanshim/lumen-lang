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
    if maxsize is None or type(maxsize) == type(1):
        if maxsize is not None and maxsize < 0:
            maxsize = 0
        return _CacheDecorator(maxsize, typed)
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

def total_ordering(cls):
    raise 'NotImplementedError: total_ordering needs object ordering methods'
