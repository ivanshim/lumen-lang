"""Weak references: a reference that does not keep its object alive.

A reference is made by the kernel's own weak hold on the object; calling
the reference gives the object back while it is still there and None
afterwards. A callback given when the reference is made is called with
the reference once the object has gone, provided the reference itself is
still alive, which is what the dictionaries and the set below build on.
"""

__all__ = ["ref", "proxy", "getweakrefcount", "getweakrefs", "WeakKeyDictionary",
           "ReferenceType", "ProxyType", "CallableProxyType", "ProxyTypes",
           "WeakValueDictionary", "WeakSet", "WeakMethod", "finalize"]


class ref:
    __callback__ = None
    _hash = None

    def __init__(self, ob, callback=None):
        self._handle = __weak_make(ob, self, callback)
        self.__callback__ = callback

    def __call__(self):
        return __weak_get(self._handle)

    def __hash__(self):
        if self._hash is None:
            ob = self()
            if ob is None:
                raise TypeError("weak object has gone away")
            self._hash = hash(ob)
        return self._hash

    def __eq__(self, other):
        if not isinstance(other, ref):
            return NotImplemented
        a = self()
        b = other()
        if a is None or b is None:
            return self is other
        return a == b

    def __ne__(self, other):
        answer = self.__eq__(other)
        if answer is NotImplemented:
            return answer
        return not answer

    def __repr__(self):
        ob = self()
        if ob is None:
            return "<weakref at %s; dead>" % hex(id(self))
        return "<weakref at %s; to '%s' at %s>" % (hex(id(self)), type(ob).__name__, hex(id(ob)))

    def __class_getitem__(cls, item):
        return cls


ReferenceType = ref


def getweakrefcount(ob):
    """How many weak references there are to ob: not kept by this kernel."""
    return 0


def getweakrefs(ob):
    """The weak references to ob: not kept by this kernel."""
    return []


class ProxyType:
    """A stand-in that behaves like its object while the object lives."""

    def __init__(self, ob, callback=None):
        self._proxied = ref(ob, callback)

    def _object(self):
        ob = self._proxied()
        if ob is None:
            raise ReferenceError("weakly-referenced object no longer exists")
        return ob

    def __getattr__(self, name):
        if name.startswith("__") and name.endswith("__"):
            raise AttributeError(name)
        return getattr(self._object(), name)

    def __repr__(self):
        ob = self._proxied()
        if ob is None:
            return "<weakproxy at %s; dead>" % hex(id(self))
        return "<weakproxy at %s; to '%s' at %s>" % (hex(id(self)), type(ob).__name__, hex(id(ob)))

    def __str__(self):
        return str(self._object())

    def __bool__(self):
        return bool(self._object())

    def __len__(self):
        return len(self._object())

    def __iter__(self):
        return iter(self._object())

    def __next__(self):
        return next(self._object())

    def __contains__(self, item):
        return item in self._object()

    def __getitem__(self, key):
        return self._object()[key]

    def __setitem__(self, key, value):
        self._object()[key] = value

    def __delitem__(self, key):
        del self._object()[key]

    def __eq__(self, other):
        return self._object() == other

    def __ne__(self, other):
        return self._object() != other

    def __lt__(self, other):
        return self._object() < other

    def __le__(self, other):
        return self._object() <= other

    def __gt__(self, other):
        return self._object() > other

    def __ge__(self, other):
        return self._object() >= other

    def __add__(self, other):
        return self._object() + other

    def __sub__(self, other):
        return self._object() - other

    def __mul__(self, other):
        return self._object() * other

    def __index__(self):
        return self._object().__index__()

    def __int__(self):
        return int(self._object())

    def __float__(self):
        return float(self._object())

    def __abs__(self):
        return abs(self._object())

    def __neg__(self):
        return -self._object()

    __hash__ = None


class CallableProxyType(ProxyType):
    def __call__(self, *args, **kwargs):
        return self._object()(*args, **kwargs)


ProxyTypes = (ProxyType, CallableProxyType)


def proxy(ob, callback=None):
    if callable(ob):
        return CallableProxyType(ob, callback)
    return ProxyType(ob, callback)


class WeakMethod(ref):
    """A weak reference to a bound method, alive while both the object
    and the function it is bound to are."""

    _alive = True

    def __init__(self, meth, callback=None):
        try:
            obj = meth.__self__
            func = meth.__func__
        except AttributeError:
            raise TypeError("argument should be a bound method, not {}".format(type(meth))) from None
        def _cb(arg):
            # The self-weakref trick is needed to avoid creating a reference
            # cycle.
            self = self_wr()
            if self is not None and self._alive:
                self._alive = False
                if callback is not None:
                    callback(self)
        super().__init__(obj, _cb)
        self._func_ref = ref(func, _cb)
        self._meth_type = type(meth)
        self_wr = ref(self)
        # A kernel whose closures keep their whole frame must not keep
        # the method's object alive through _cb.
        del meth, obj, func

    def __call__(self):
        obj = super().__call__()
        func = self._func_ref()
        if obj is None or func is None:
            return None
        try:
            return self._meth_type(func, obj)
        except NotImplementedError:
            return getattr(obj, func.__name__)

    def __eq__(self, other):
        if isinstance(other, WeakMethod):
            if not self._alive or not other._alive:
                return self is other
            return ref.__eq__(self, other) and self._func_ref == other._func_ref
        return NotImplemented

    def __ne__(self, other):
        if isinstance(other, WeakMethod):
            if not self._alive or not other._alive:
                return self is not other
            return ref.__ne__(self, other) or self._func_ref != other._func_ref
        return NotImplemented

    __hash__ = ref.__hash__


class KeyedRef(ref):
    """A reference that remembers the key it is filed under."""

    def __init__(self, ob, callback, key):
        super().__init__(ob, callback)
        self.key = key


def _remove_dead_weakref(d, key):
    wr = d.get(key)
    if wr is not None and wr() is None:
        del d[key]


class WeakValueDictionary:
    """Mapping class that references values weakly.

    Entries in the dictionary will be discarded when no strong
    reference to the value exists anymore
    """

    def __init__(self, other=(), **kw):
        def remove(wr, selfref=ref(self)):
            self = selfref()
            if self is not None:
                if self._iterating:
                    self._pending_removals.append(wr.key)
                else:
                    _remove_dead_weakref(self.data, wr.key)
        self._remove = remove
        self._pending_removals = []
        self._iterating = set()
        self.data = {}
        self.update(other, **kw)

    def _commit_removals(self):
        pop = self._pending_removals.pop
        d = self.data
        while True:
            try:
                key = pop()
            except IndexError:
                return
            _remove_dead_weakref(d, key)

    def __getitem__(self, key):
        if self._pending_removals:
            self._commit_removals()
        o = self.data[key]()
        if o is None:
            raise KeyError(key)
        else:
            return o

    def __delitem__(self, key):
        if self._pending_removals:
            self._commit_removals()
        del self.data[key]

    def __len__(self):
        if self._pending_removals:
            self._commit_removals()
        return len(self.data)

    def __contains__(self, key):
        if self._pending_removals:
            self._commit_removals()
        try:
            o = self.data[key]()
        except KeyError:
            return False
        return o is not None

    def __repr__(self):
        return "<%s at %s>" % (self.__class__.__name__, hex(id(self)))

    def __setitem__(self, key, value):
        if self._pending_removals:
            self._commit_removals()
        self.data[key] = KeyedRef(value, self._remove, key)

    def copy(self):
        if self._pending_removals:
            self._commit_removals()
        new = WeakValueDictionary()
        for key, wr in self.data.items():
            o = wr()
            if o is not None:
                new[key] = o
        return new

    __copy__ = copy

    def get(self, key, default=None):
        if self._pending_removals:
            self._commit_removals()
        try:
            wr = self.data[key]
        except KeyError:
            return default
        else:
            o = wr()
            if o is None:
                return default
            else:
                return o

    def items(self):
        if self._pending_removals:
            self._commit_removals()
        for k, wr in list(self.data.items()):
            v = wr()
            if v is not None:
                yield k, v

    def keys(self):
        if self._pending_removals:
            self._commit_removals()
        for k, wr in list(self.data.items()):
            if wr() is not None:
                yield k

    __iter__ = keys

    def itervaluerefs(self):
        if self._pending_removals:
            self._commit_removals()
        yield from list(self.data.values())

    def values(self):
        if self._pending_removals:
            self._commit_removals()
        for wr in list(self.data.values()):
            obj = wr()
            if obj is not None:
                yield obj

    def popitem(self):
        if self._pending_removals:
            self._commit_removals()
        while True:
            key, wr = self.data.popitem()
            o = wr()
            if o is not None:
                return key, o

    def pop(self, key, *args):
        if self._pending_removals:
            self._commit_removals()
        try:
            o = self.data.pop(key)()
        except KeyError:
            o = None
        if o is None:
            if args:
                return args[0]
            else:
                raise KeyError(key)
        else:
            return o

    def setdefault(self, key, default=None):
        try:
            o = self.data[key]()
        except KeyError:
            o = None
        if o is None:
            if self._pending_removals:
                self._commit_removals()
            self.data[key] = KeyedRef(default, self._remove, key)
            return default
        else:
            return o

    def update(self, other=None, **kwargs):
        if self._pending_removals:
            self._commit_removals()
        d = self.data
        if other is not None:
            if not hasattr(other, "items"):
                other = dict(other)
            for key, o in other.items():
                d[key] = KeyedRef(o, self._remove, key)
        for key, o in kwargs.items():
            d[key] = KeyedRef(o, self._remove, key)

    def valuerefs(self):
        if self._pending_removals:
            self._commit_removals()
        return list(self.data.values())

    def __eq__(self, other):
        if isinstance(other, WeakValueDictionary):
            return dict(self.items()) == dict(other.items())
        if isinstance(other, dict):
            return dict(self.items()) == other
        return NotImplemented

    def clear(self):
        self.data.clear()


class WeakKeyDictionary:
    """Mapping class that references keys weakly.

    Entries in the dictionary will be discarded when there is no
    longer a strong reference to the key.
    """

    def __init__(self, dict=None):
        self.data = {}
        def remove(k, selfref=ref(self)):
            self = selfref()
            if self is not None:
                if self._iterating:
                    self._pending_removals.append(k)
                else:
                    try:
                        del self.data[k]
                    except KeyError:
                        pass
        self._remove = remove
        self._pending_removals = []
        self._iterating = set()
        self._dirty_len = False
        if dict is not None:
            self.update(dict)

    def _commit_removals(self):
        pop = self._pending_removals.pop
        d = self.data
        while True:
            try:
                key = pop()
            except IndexError:
                return
            try:
                del d[key]
            except KeyError:
                pass

    def _scrub_removals(self):
        d = self.data
        self._pending_removals = [k for k in self._pending_removals if k in d]
        self._dirty_len = False

    def __delitem__(self, key):
        self._dirty_len = True
        del self.data[ref(key)]

    def __getitem__(self, key):
        return self.data[ref(key)]

    def __len__(self):
        if self._dirty_len and self._pending_removals:
            self._scrub_removals()
        return len(self.data) - len(self._pending_removals)

    def __repr__(self):
        return "<%s at %s>" % (self.__class__.__name__, hex(id(self)))

    def __setitem__(self, key, value):
        self.data[ref(key, self._remove)] = value

    def copy(self):
        new = WeakKeyDictionary()
        for key, value in self.data.items():
            o = key()
            if o is not None:
                new[o] = value
        return new

    __copy__ = copy

    def get(self, key, default=None):
        return self.data.get(ref(key), default)

    def __contains__(self, key):
        try:
            wr = ref(key)
        except TypeError:
            return False
        return wr in self.data

    def items(self):
        for wr, value in list(self.data.items()):
            key = wr()
            if key is not None:
                yield key, value

    def keys(self):
        for wr in list(self.data):
            obj = wr()
            if obj is not None:
                yield obj

    __iter__ = keys

    def values(self):
        for wr, value in list(self.data.items()):
            if wr() is not None:
                yield value

    def keyrefs(self):
        return list(self.data)

    def popitem(self):
        self._dirty_len = True
        while True:
            key, value = self.data.popitem()
            o = key()
            if o is not None:
                return o, value

    def pop(self, key, *args):
        self._dirty_len = True
        return self.data.pop(ref(key), *args)

    def setdefault(self, key, default=None):
        return self.data.setdefault(ref(key, self._remove), default)

    def update(self, dict=None, **kwargs):
        d = self.data
        if dict is not None:
            if not hasattr(dict, "items"):
                dict = type({})(dict)
            for key, value in dict.items():
                d[ref(key, self._remove)] = value
        if len(kwargs):
            self.update(kwargs)

    def __eq__(self, other):
        if isinstance(other, WeakKeyDictionary):
            return dict(self.items()) == dict(other.items())
        return NotImplemented

    def clear(self):
        self.data.clear()


class WeakSet:
    def __init__(self, data=None):
        self.data = set()
        def _remove(item, selfref=ref(self)):
            self = selfref()
            if self is not None:
                if self._iterating:
                    self._pending_removals.append(item)
                else:
                    self.data.discard(item)
        self._remove = _remove
        self._pending_removals = []
        self._iterating = set()
        if data is not None:
            self.update(data)

    def _commit_removals(self):
        pop = self._pending_removals.pop
        discard = self.data.discard
        while True:
            try:
                item = pop()
            except IndexError:
                return
            discard(item)

    def __iter__(self):
        for itemref in list(self.data):
            item = itemref()
            if item is not None:
                yield item

    def __len__(self):
        return len(self.data) - len(self._pending_removals)

    def __contains__(self, item):
        try:
            wr = ref(item)
        except TypeError:
            return False
        return wr in self.data

    def add(self, item):
        if self._pending_removals:
            self._commit_removals()
        self.data.add(ref(item, self._remove))

    def clear(self):
        if self._pending_removals:
            self._commit_removals()
        self.data.clear()

    def copy(self):
        return self.__class__(self)

    def pop(self):
        if self._pending_removals:
            self._commit_removals()
        while True:
            try:
                itemref = self.data.pop()
            except KeyError:
                raise KeyError('pop from empty WeakSet') from None
            item = itemref()
            if item is not None:
                return item

    def remove(self, item):
        if self._pending_removals:
            self._commit_removals()
        self.data.remove(ref(item))

    def discard(self, item):
        if self._pending_removals:
            self._commit_removals()
        self.data.discard(ref(item))

    def update(self, other):
        if self._pending_removals:
            self._commit_removals()
        for element in other:
            self.add(element)

    def __ior__(self, other):
        self.update(other)
        return self

    def difference(self, other):
        newset = self.copy()
        newset.difference_update(other)
        return newset
    __sub__ = difference

    def difference_update(self, other):
        self.__isub__(other)

    def __isub__(self, other):
        if self._pending_removals:
            self._commit_removals()
        if self is other:
            self.data.clear()
        else:
            self.data.difference_update(ref(item) for item in other)
        return self

    def intersection(self, other):
        return self.__class__(item for item in other if item in self)
    __and__ = intersection

    def intersection_update(self, other):
        self.__iand__(other)

    def __iand__(self, other):
        if self._pending_removals:
            self._commit_removals()
        self.data.intersection_update(ref(item) for item in other)
        return self

    def issubset(self, other):
        return self.data.issubset(ref(item) for item in other)
    __le__ = issubset

    def __lt__(self, other):
        return self.data < set(map(ref, other))

    def issuperset(self, other):
        return self.data.issuperset(ref(item) for item in other)
    __ge__ = issuperset

    def __gt__(self, other):
        return self.data > set(map(ref, other))

    def __eq__(self, other):
        if not isinstance(other, self.__class__):
            return NotImplemented
        return self.data == set(map(ref, other))

    def symmetric_difference(self, other):
        newset = self.copy()
        newset.symmetric_difference_update(other)
        return newset
    __xor__ = symmetric_difference

    def symmetric_difference_update(self, other):
        self.__ixor__(other)

    def __ixor__(self, other):
        if self._pending_removals:
            self._commit_removals()
        if self is other:
            self.data.clear()
        else:
            self.data.symmetric_difference_update(ref(item, self._remove) for item in other)
        return self

    def union(self, other):
        return self.__class__(e for s in (self, other) for e in s)
    __or__ = union

    def isdisjoint(self, other):
        return len(self.intersection(other)) == 0

    def __repr__(self):
        return repr(self.data)


class finalize:
    """Class for finalization of weakrefable objects

    finalize(obj, func, *args, **kwargs) returns a callable finalizer
    object which will be called when obj is garbage collected. The
    first time the finalizer is called it evaluates func(*arg, **kwargs)
    and returns the result. After this the finalizer is dead, and
    calling it just returns None.
    """

    _registry = {}
    _shutdown = False
    _index = 0

    class _Info:
        pass

    def __init__(self, obj, func, *args, **kwargs):
        info = self._Info()
        info.weakref = ref(obj, self)
        info.func = func
        info.args = args
        info.kwargs = kwargs or None
        info.atexit = True
        finalize._index += 1
        info.index = finalize._index
        self._registry[self] = info

    def __call__(self, _=None):
        """If alive then mark as dead and return func(*args, **kwargs);
        otherwise return None"""
        info = self._registry.pop(self, None)
        if info and not self._shutdown:
            return info.func(*info.args, **(info.kwargs or {}))

    def detach(self):
        """If alive then mark as dead and return (obj, func, args, kwargs);
        otherwise return None"""
        info = self._registry.get(self)
        obj = info and info.weakref()
        if obj is not None and self._registry.pop(self, None):
            return (obj, info.func, info.args, info.kwargs or {})

    def peek(self):
        """If alive then return (obj, func, args, kwargs);
        otherwise return None"""
        info = self._registry.get(self)
        obj = info and info.weakref()
        if obj is not None:
            return (obj, info.func, info.args, info.kwargs or {})

    @property
    def alive(self):
        """Whether finalizer is alive"""
        return self in self._registry

    @property
    def atexit(self):
        """Whether finalizer should be called at exit"""
        info = self._registry.get(self)
        return bool(info) and info.atexit

    @atexit.setter
    def atexit(self, value):
        info = self._registry.get(self)
        if info:
            info.atexit = bool(value)

    def __repr__(self):
        info = self._registry.get(self)
        obj = info and info.weakref()
        if obj is None:
            return '<%s object at %s; dead>' % (type(self).__name__, hex(id(self)))
        else:
            return '<%s object at %s; for %r at %s>' % \
                (type(self).__name__, hex(id(self)), type(obj).__name__, hex(id(obj)))

    @classmethod
    def _select_for_exit(cls):
        L = [(f, i) for (f, i) in cls._registry.items() if i.atexit]
        L.sort(key=lambda item: item[1].index)
        return [f for (f, i) in L]

    @classmethod
    def _exitfunc(cls):
        reenable_gc = False
        try:
            if cls._registry:
                import gc
                if gc.isenabled():
                    reenable_gc = True
                    gc.disable()
                pending = None
                while True:
                    if pending is None or finalize._dirty:
                        pending = cls._select_for_exit()
                        finalize._dirty = False
                    if not pending:
                        break
                    f = pending.pop()
                    try:
                        f()
                    except Exception:
                        import sys
                        print(sys.exc_info()[1], file=sys.stderr)
        finally:
            if reenable_gc:
                gc.enable()

    _dirty = False
