# Function metadata and dispatch helpers.
def _identity(function):
    return function

WRAPPER_ASSIGNMENTS = ('__module__', '__name__', '__qualname__', '__doc__',
                       '__annotate__', '__type_params__')
WRAPPER_UPDATES = ('__dict__',)
def wraps(wrapped,
          assigned = WRAPPER_ASSIGNMENTS,
          updated = WRAPPER_UPDATES):
    """Decorator factory to apply update_wrapper() to a wrapper function

       Returns a decorator that invokes update_wrapper() with the decorated
       function as the wrapper argument and the arguments to wraps() as the
       remaining arguments. Default arguments are as for update_wrapper().
       This is a convenience function to simplify applying partial() to
       update_wrapper().
    """
    return partial(update_wrapper, wrapped=wrapped,
                   assigned=assigned, updated=updated)

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

# Dispatch and wrapper helpers from CPython 3b564385e4c9; PSF License.
from abc import get_cache_token
from types import MappingProxyType, GenericAlias, UnionType
def update_wrapper(wrapper,
                   wrapped,
                   assigned = WRAPPER_ASSIGNMENTS,
                   updated = WRAPPER_UPDATES):
    """Update a wrapper function to look like the wrapped function

       wrapper is the function to be updated
       wrapped is the original function
       assigned is a tuple naming the attributes assigned directly
       from the wrapped function to the wrapper function (defaults to
       functools.WRAPPER_ASSIGNMENTS)
       updated is a tuple naming the attributes of the wrapper that
       are updated with the corresponding attribute from the wrapped
       function (defaults to functools.WRAPPER_UPDATES)
    """
    for attr in assigned:
        try:
            value = getattr(wrapped, attr)
        except AttributeError:
            pass
        else:
            setattr(wrapper, attr, value)
    for attr in updated:
        getattr(wrapper, attr).update(getattr(wrapped, attr, {}))
    # Issue #17482: set __wrapped__ last so we don't inadvertently copy it
    # from the wrapped function when updating __dict__
    wrapper.__wrapped__ = wrapped
    # Return the wrapper so this can be used as a decorator via partial()
    return wrapper

def _c3_merge(sequences):
    """Merges MROs in *sequences* to a single MRO using the C3 algorithm.

    Adapted from https://docs.python.org/3/howto/mro.html.

    """
    result = []
    while True:
        sequences = [s for s in sequences if s]   # purge empty sequences
        if not sequences:
            return result
        for s1 in sequences:   # find merge candidates among seq heads
            candidate = s1[0]
            for s2 in sequences:
                if candidate in s2[1:]:
                    candidate = None
                    break      # reject the current head, it appears later
            else:
                break
        if candidate is None:
            raise RuntimeError("Inconsistent hierarchy")
        result.append(candidate)
        # remove the chosen candidate
        for seq in sequences:
            if seq[0] == candidate:
                del seq[0]

def _c3_mro(cls, abcs=None):
    """Computes the method resolution order using extended C3 linearization.

    If no *abcs* are given, the algorithm works exactly like the built-in C3
    linearization used for method resolution.

    If given, *abcs* is a list of abstract base classes that should be inserted
    into the resulting MRO. Unrelated ABCs are ignored and don't end up in the
    result. The algorithm inserts ABCs where their functionality is introduced,
    i.e. issubclass(cls, abc) returns True for the class itself but returns
    False for all its direct base classes. Implicit ABCs for a given class
    (either registered or inferred from the presence of a special method like
    __len__) are inserted directly after the last ABC explicitly listed in the
    MRO of said class. If two implicit ABCs end up next to each other in the
    resulting MRO, their ordering depends on the order of types in *abcs*.

    """
    for i, base in enumerate(reversed(cls.__bases__)):
        if hasattr(base, '__abstractmethods__'):
            boundary = len(cls.__bases__) - i
            break   # Bases up to the last explicit ABC are considered first.
    else:
        boundary = 0
    abcs = list(abcs) if abcs else []
    explicit_bases = list(cls.__bases__[:boundary])
    abstract_bases = []
    other_bases = list(cls.__bases__[boundary:])
    for base in abcs:
        if issubclass(cls, base) and not any(
                issubclass(b, base) for b in cls.__bases__
            ):
            # If *cls* is the class that introduces behaviour described by
            # an ABC *base*, insert said ABC to its MRO.
            abstract_bases.append(base)
    for base in abstract_bases:
        abcs.remove(base)
    explicit_c3_mros = [_c3_mro(base, abcs=abcs) for base in explicit_bases]
    abstract_c3_mros = [_c3_mro(base, abcs=abcs) for base in abstract_bases]
    other_c3_mros = [_c3_mro(base, abcs=abcs) for base in other_bases]
    return _c3_merge(
        [[cls]] +
        explicit_c3_mros + abstract_c3_mros + other_c3_mros +
        [explicit_bases] + [abstract_bases] + [other_bases]
    )

def _compose_mro(cls, types):
    """Calculates the method resolution order for a given class *cls*.

    Includes relevant abstract base classes (with their respective bases) from
    the *types* iterable. Uses a modified C3 linearization algorithm.

    """
    bases = set(cls.__mro__)
    # Remove entries which are already present in the __mro__ or unrelated.
    def is_related(typ):
        return (typ not in bases and hasattr(typ, '__mro__')
                                 and not isinstance(typ, GenericAlias)
                                 and issubclass(cls, typ))
    types = [n for n in types if is_related(n)]
    # Remove entries which are strict bases of other entries (they will end up
    # in the MRO anyway.
    def is_strict_base(typ):
        for other in types:
            if typ != other and typ in other.__mro__:
                return True
        return False
    types = [n for n in types if not is_strict_base(n)]
    # Subclasses of the ABCs in *types* which are also implemented by
    # *cls* can be used to stabilize ABC ordering.
    type_set = set(types)
    mro = []
    for typ in types:
        found = []
        for sub in typ.__subclasses__():
            if sub not in bases and issubclass(cls, sub):
                found.append([s for s in sub.__mro__ if s in type_set])
        if not found:
            mro.append(typ)
            continue
        # Favor subclasses with the biggest number of useful bases
        found.sort(key=len, reverse=True)
        for sub in found:
            for subcls in sub:
                if subcls not in mro:
                    mro.append(subcls)
    return _c3_mro(cls, abcs=mro)

def _find_impl(cls, registry):
    """Returns the best matching implementation from *registry* for type *cls*.

    Where there is no registered implementation for a specific type, its method
    resolution order is used to find a more generic implementation.

    Note: if *registry* does not contain an implementation for the base
    *object* type, this function may return None.

    """
    mro = _compose_mro(cls, registry.keys())
    match = None
    for t in mro:
        if match is not None:
            # If *match* is an implicit ABC but there is another unrelated,
            # equally matching implicit ABC, refuse the temptation to guess.
            if (t in registry and t not in cls.__mro__
                              and match not in cls.__mro__
                              and not issubclass(match, t)):
                raise RuntimeError("Ambiguous dispatch: {} or {}".format(
                    match, t))
            break
        if t in registry:
            match = t
    return registry.get(match)

def singledispatch(func):
    """Single-dispatch generic function decorator.

    Transforms a function into a generic function, which can have different
    behaviours depending upon the type of its first argument. The decorated
    function acts as the default implementation, and additional
    implementations can be registered using the register() attribute of the
    generic function.
    """
    # There are many programs that use functools without singledispatch, so we
    # trade-off making singledispatch marginally slower for the benefit of
    # making start-up of such applications slightly faster.
    import weakref

    registry = {}
    dispatch_cache = weakref.WeakKeyDictionary()
    cache_token = None

    def dispatch(cls):
        """generic_func.dispatch(cls) -> <function implementation>

        Runs the dispatch algorithm to return the best available implementation
        for the given *cls* registered on *generic_func*.

        """
        nonlocal cache_token
        if cache_token is not None:
            current_token = get_cache_token()
            if cache_token != current_token:
                dispatch_cache.clear()
                cache_token = current_token
        try:
            impl = dispatch_cache[cls]
        except KeyError:
            try:
                impl = registry[cls]
            except KeyError:
                impl = _find_impl(cls, registry)
            dispatch_cache[cls] = impl
        return impl

    def _is_valid_dispatch_type(cls):
        if isinstance(cls, type):
            return True
        return (isinstance(cls, UnionType) and
                all(isinstance(arg, type) for arg in cls.__args__))

    def register(cls, func=None):
        """generic_func.register(cls, func) -> func

        Registers a new implementation for the given *cls* on a *generic_func*.

        """
        nonlocal cache_token
        if _is_valid_dispatch_type(cls):
            if func is None:
                return lambda f: register(cls, f)
        else:
            if func is not None:
                raise TypeError(
                    f"Invalid first argument to `register()`. "
                    f"{cls!r} is not a class or union type."
                )
            ann = getattr(cls, '__annotate__', None)
            if ann is None:
                raise TypeError(
                    f"Invalid first argument to `register()`: {cls!r}. "
                    f"Use either `@register(some_class)` or plain `@register` "
                    f"on an annotated function."
                )
            func = cls

            # only import typing if annotation parsing is necessary
            from typing import get_type_hints
            from annotationlib import Format, ForwardRef
            argname, cls = next(iter(get_type_hints(func, format=Format.FORWARDREF).items()))
            if not _is_valid_dispatch_type(cls):
                if isinstance(cls, UnionType):
                    raise TypeError(
                        f"Invalid annotation for {argname!r}. "
                        f"{cls!r} not all arguments are classes."
                    )
                elif isinstance(cls, ForwardRef):
                    raise TypeError(
                        f"Invalid annotation for {argname!r}. "
                        f"{cls!r} is an unresolved forward reference."
                    )
                else:
                    raise TypeError(
                        f"Invalid annotation for {argname!r}. "
                        f"{cls!r} is not a class."
                    )

        if isinstance(cls, UnionType):
            for arg in cls.__args__:
                registry[arg] = func
        else:
            registry[cls] = func
        if cache_token is None and hasattr(cls, '__abstractmethods__'):
            cache_token = get_cache_token()
        dispatch_cache.clear()
        return func

    def wrapper(*args, **kw):
        if not args:
            raise TypeError(f'{funcname} requires at least '
                            '1 positional argument')
        return dispatch(args[0].__class__)(*args, **kw)

    funcname = getattr(func, '__name__', 'singledispatch function')
    registry[object] = func
    wrapper.register = register
    wrapper.dispatch = dispatch
    wrapper.registry = MappingProxyType(registry)
    wrapper._clear_cache = dispatch_cache.clear
    update_wrapper(wrapper, func)
    return wrapper


# Descriptor version


def cmp_to_key(mycmp):
    class K:
        def __init__(self, obj):
            self.obj = obj
        def __lt__(self, other):
            return mycmp(self.obj, other.obj) < 0
        def __gt__(self, other):
            return mycmp(self.obj, other.obj) > 0
        def __eq__(self, other):
            return mycmp(self.obj, other.obj) == 0
        def __le__(self, other):
            return mycmp(self.obj, other.obj) <= 0
        def __ge__(self, other):
            return mycmp(self.obj, other.obj) >= 0
    return K

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
