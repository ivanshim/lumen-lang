# Runtime adapter derived from CPython Lib/functools.py at v3.14.8 / 8e6e75d9102e; PSF License.
# Python implementation of the _functools accelerator interface.
"""Tools that make up the _functools accelerator: partial, Placeholder, reduce,
cmp_to_key and the LRU cache wrapper."""

from operator import itemgetter
from reprlib import recursive_repr
from types import GenericAlias, MethodType
from _thread import RLock

__all__ = ['Placeholder', 'cmp_to_key', 'partial', 'reduce']


class _builtin_function:
    """A plain callable that, like a C function, never binds as a method."""
    def __init__(self, func):
        self._func = func
        self.__name__ = self.__qualname__ = func.__name__
        self.__doc__ = func.__doc__
        self.__module__ = 'functools'

    def __call__(self, /, *args, **kwds):
        return self._func(*args, **kwds)

    def __repr__(self):
        return f"<built-in function {self.__name__}>"

    def __reduce__(self):
        return self.__qualname__


class _PlaceholderType:
    """The type of the Placeholder singleton.

    Used as a placeholder for partial arguments.
    """
    __instance = None
    __slots__ = ()

    def __init_subclass__(cls, *args, **kwargs):
        raise TypeError(f"type '{cls.__name__}' is not an acceptable base type")

    def __new__(cls):
        if cls.__instance is None:
            cls.__instance = object.__new__(cls)
        return cls.__instance

    def __repr__(self):
        return 'Placeholder'

    def __reduce__(self):
        return 'Placeholder'

_PlaceholderType.__module__ = 'functools'
Placeholder = _PlaceholderType()


def _prepare_merger(args):
    if not args:
        return 0, None
    nargs = len(args)
    order = []
    j = nargs
    for i, a in enumerate(args):
        if a is Placeholder:
            order.append(j)
            j += 1
        else:
            order.append(i)
    phcount = j - nargs
    merger = itemgetter(*order) if phcount else None
    return phcount, merger


def _partial_new(cls, func, /, *args, **keywords):
    if not callable(func):
        raise TypeError("the first argument must be callable")
    if args and args[-1] is Placeholder:
        raise TypeError("trailing Placeholders are not allowed")
    for value in keywords.values():
        if value is Placeholder:
            raise TypeError("Placeholder cannot be passed as a keyword argument")
    if isinstance(func, partial):
        pto_phcount = func._phcount
        tot_args = func._args
        if args:
            tot_args += args
            if pto_phcount:
                nargs = len(args)
                if nargs < pto_phcount:
                    tot_args += (Placeholder,) * (pto_phcount - nargs)
                tot_args = func._merger(tot_args)
                if nargs > pto_phcount:
                    tot_args += args[pto_phcount:]
            phcount, merger = _prepare_merger(tot_args)
        else:
            phcount, merger = pto_phcount, func._merger
        keywords = {**func._keywords, **keywords}
        func = func._func
    else:
        tot_args = args
        phcount, merger = _prepare_merger(tot_args)

    self = object.__new__(cls)
    self._func = func
    self._args = tot_args
    self._keywords = keywords
    self._phcount = phcount
    self._merger = merger
    return self


def _partial_repr(self):
    # Snapshot the state first: a nested repr may call __setstate__ on self.
    cls = type(self)
    func, args, keywords = self._func, self._args, self._keywords
    parts = [repr(func)]
    parts.extend(map(repr, args))
    parts.extend(f"{k}={v!r}" for k, v in list(keywords.items()))
    return f"{cls.__module__}.{cls.__qualname__}({', '.join(parts)})"


class partial:
    """Create a new function with partial application of the given arguments
    and keywords.
    """

    __slots__ = ("_func", "_args", "_keywords", "_phcount", "_merger",
                 "__dict__", "__weakref__")

    __new__ = _partial_new
    __repr__ = recursive_repr()(_partial_repr)

    func = property(lambda self: self._func, doc="function object to use in future partial calls")
    args = property(lambda self: self._args, doc="tuple of arguments to future partial calls")
    keywords = property(lambda self: self._keywords, doc="dictionary of keyword arguments to future partial calls")

    def __call__(self, /, *args, **keywords):
        phcount = self._phcount
        if phcount:
            try:
                pto_args = self._merger(self._args + args)
                args = args[phcount:]
            except IndexError:
                raise TypeError("missing positional arguments "
                                "in 'partial' call; expected "
                                f"at least {phcount}, got {len(args)}")
        else:
            pto_args = self._args
        keywords = {**self._keywords, **keywords}
        return self._func(*pto_args, *args, **keywords)

    def __delattr__(self, name):
        if name == '__dict__':
            raise TypeError("cannot delete __dict__")
        object.__delattr__(self, name)

    def __get__(self, obj, objtype=None):
        if obj is None:
            return self
        return MethodType(self, obj)

    def __reduce__(self):
        return type(self), (self._func,), (self._func, self._args,
               self._keywords or None, self.__dict__ or None)

    def __setstate__(self, state):
        if not isinstance(state, tuple):
            raise TypeError("argument to __setstate__ must be a tuple")
        if len(state) != 4:
            raise TypeError(f"expected 4 items in state, got {len(state)}")
        func, args, kwds, namespace = state
        if (not callable(func) or not isinstance(args, tuple) or
           (kwds is not None and not isinstance(kwds, dict)) or
           (namespace is not None and not isinstance(namespace, dict))):
            raise TypeError("invalid partial state")

        if args and args[-1] is Placeholder:
            raise TypeError("trailing Placeholders are not allowed")
        phcount, merger = _prepare_merger(args)

        args = tuple(args)
        if kwds is None:
            kwds = {}
        elif type(kwds) is not dict:
            kwds = dict(kwds)
        if namespace is None:
            namespace = {}

        self.__dict__ = namespace
        self._func = func
        self._args = args
        self._keywords = kwds
        self._phcount = phcount
        self._merger = merger

    __class_getitem__ = classmethod(GenericAlias)

partial.__module__ = 'functools'


_initial_missing = object()

def reduce(function, iterable, /, initial=_initial_missing):
    """reduce(function, iterable[, initial], /) -> value

Apply a function of two arguments cumulatively to the items of an iterable, from left to right.

This effectively reduces the iterable to a single value.  If initial is present,
it is placed before the items of the iterable in the calculation, and serves as
a default when the iterable is empty.

For example, reduce(lambda x, y: x+y, [1, 2, 3, 4, 5])
calculates ((((1 + 2) + 3) + 4) + 5).
"""
    it = iter(iterable)
    if initial is _initial_missing:
        try:
            value = next(it)
        except StopIteration:
            raise TypeError(
                "reduce() of empty iterable with no initial value") from None
    else:
        value = initial
    for element in it:
        value = function(value, element)
    return value

reduce = _builtin_function(reduce)


def cmp_to_key(mycmp):
    """Convert a cmp= function into a key= function."""
    class K(object):
        __slots__ = ['obj']
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
        __hash__ = None
    return K

cmp_to_key = _builtin_function(cmp_to_key)


_missing = object()
_kwd_mark = (object(),)
_fasttypes = {int, str}

def _make_key(args, kwds, typed):
    key = args
    if kwds:
        key += _kwd_mark
        for item in kwds.items():
            key += item
    if typed:
        key += tuple(type(v) for v in args)
        if kwds:
            key += tuple(type(v) for v in kwds.values())
    elif key.__len__() == 1 and type(key[0]) in _fasttypes:
        return key[0]
    return key


class _lru_cache_wrapper:
    """Create a cached callable that wraps another function.

user_function:      the function being cached

maxsize:  0         for no caching
          None      for unlimited cache size
          n         for a bounded cache

typed:    False     cache f(3) and f(3.0) as identical calls
          True      cache f(3) and f(3.0) as distinct calls

cache_info_type:    namedtuple class with the fields:
                        hits misses currsize maxsize
"""

    __slots__ = ("_user_function", "_maxsize", "_typed", "_info_type", "_lock", "_cache", "_hits", "_misses", "_full", "_root", "__dict__", "__weakref__")

    def __init__(self, user_function, maxsize, typed, cache_info_type):
        if not callable(user_function):
            raise TypeError("the first argument must be callable")
        if maxsize is not None and not isinstance(maxsize, int):
            raise TypeError("maxsize should be integer or None")
        if maxsize is not None and maxsize < 0:
            maxsize = 0
        self.__wrapped__ = user_function
        self._user_function = user_function   # private: __wrapped__ may be reassigned
        self._maxsize = maxsize
        self._typed = bool(typed)
        self._info_type = cache_info_type
        self._lock = RLock()
        self._reset()

    def _reset(self):
        # In place: a call still running holds the old dict and root.
        cache = getattr(self, '_cache', None)
        if cache is None:
            cache = self._cache = {}
        cache.clear()
        self._hits = self._misses = 0
        self._full = False
        root = getattr(self, '_root', None)
        if root is None:
            root = self._root = []
        root[:] = [root, root, None, None]

    def __call__(self, /, *args, **kwds):
        user_function = self._user_function
        maxsize = self._maxsize
        if maxsize == 0:
            self._misses += 1
            return user_function(*args, **kwds)
        key = _make_key(args, kwds, self._typed)
        cache = self._cache
        if maxsize is None:
            result = cache.get(key, _missing)
            if result is not _missing:
                self._hits += 1
                return result
            self._misses += 1
            result = user_function(*args, **kwds)
            cache[key] = result
            return result
        with self._lock:
            link = cache.get(key)
            if link is not None:
                # Move the link to the back of the circular queue.
                link_prev, link_next, _key, result = link
                link_prev[1] = link_next
                link_next[0] = link_prev
                root = self._root
                last = root[0]
                last[1] = root[0] = link
                link[0] = last
                link[1] = root
                self._hits += 1
                return result
            self._misses += 1
        result = user_function(*args, **kwds)
        with self._lock:
            if key in cache:
                pass
            elif self._full:
                oldroot = self._root
                oldroot[2] = key
                oldroot[3] = result
                root = self._root = oldroot[1]
                oldkey = root[2]
                oldresult = root[3]
                root[2] = root[3] = None
                del cache[oldkey]
                cache[key] = oldroot
            else:
                root = self._root
                last = root[0]
                link = [last, root, key, result]
                last[1] = root[0] = cache[key] = link
                self._full = (cache.__len__() >= maxsize)
        return result

    def cache_info(self):
        """Report cache statistics"""
        with self._lock:
            return self._info_type(self._hits, self._misses,
                                   self._maxsize, self._cache.__len__())

    def cache_clear(self):
        """Clear the cache and cache statistics"""
        with self._lock:
            self._reset()

    def __get__(self, obj, objtype=None):
        if obj is None:
            return self
        return MethodType(self, obj)

    def __copy__(self):
        return self

    def __deepcopy__(self, memo):
        return self

    def __reduce__(self):
        return self.__qualname__

_lru_cache_wrapper.__module__ = 'functools'
