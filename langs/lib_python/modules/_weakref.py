# Runtime bindings for CPython v3.14.8 Objects/weakrefobject.c and Modules/_weakref.c.
# Weak allocation, interning, retrieval, callback state, hash caching and equality
# are supplied by the Python-only weak primitives in each kernel.

_make_weak = __weak_make
_get_weak = __weak_get

class ReferenceType:
    __slots__ = ()
    __module__ = 'weakref'

    def __call__(self):
        return _get_weak(self, 'call')

    @property
    def __callback__(self):
        return _get_weak(self, 'callback')

    def __repr__(self):
        ob = self()
        if ob is None:
            return "<weakref at %s; dead>" % hex(id(self))
        try:
            name = getattr(ob, '__name__', None)
        except Exception:
            name = None
        if isinstance(name, str):
            return "<weakref at %s; to '%s' at %s (%s)>" % (hex(id(self)), type(ob).__name__, hex(id(ob)), name)
        return "<weakref at %s; to '%s' at %s>" % (hex(id(self)), type(ob).__name__, hex(id(ob)))

    __class_getitem__ = classmethod(__import__('types').GenericAlias)

# Native slots validate arguments without temporary Python frames.
# Subclass overrides still use normal lookup.
ReferenceType.__new__ = staticmethod(_get_weak(ReferenceType, 'allocator', _make_weak))
ReferenceType.__init__ = _get_weak(ReferenceType, 'initializer')
ReferenceType.__hash__ = _get_weak(ReferenceType, 'operation', '__hash__')
ReferenceType.__eq__ = _get_weak(ReferenceType, 'operation', '__eq__')
ReferenceType.__ne__ = _get_weak(ReferenceType, 'operation', '__ne__')

ref = ReferenceType


def getweakrefs(ob, /):
    return _get_weak(ob, 'refs')


def getweakrefcount(ob, /):
    return len(_get_weak(ob, 'refs'))


def _remove_dead_weakref(d, key, /):
    return _get_weak(d, 'remove', key)


class ProxyType:
    __module__ = 'weakref'
    """Proxy to an object held weakly by the runtime."""

    __slots__ = ()
    def __new__(cls, *args, **kwargs):
        raise TypeError("cannot create 'weakref." + cls.__name__ + "' instances")

    def _object(self):
        return _get_weak(self, 'proxy')

    @property
    def __class__(self):
        return type(_proxy_target(self))

    def __getattr__(self, name):
        if name.startswith("__") and name.endswith("__"):
            raise AttributeError(name)
        return getattr(_proxy_target(self), name)

    def __repr__(self):
        ob = _get_weak(self, 'call')
        if ob is None:
            return "<weakproxy at %s; dead>" % hex(id(self))
        return "<weakproxy at %s; to '%s' at %s>" % (hex(id(self)), type(ob).__name__, hex(id(ob)))

    def __str__(self):
        return str(_proxy_target(self))

    def __bool__(self):
        return bool(_proxy_target(self))

    def __len__(self):
        return len(_proxy_target(self))

    def __iter__(self):
        return iter(_proxy_target(self))

    def __next__(self):
        target = _proxy_target(self)
        if not hasattr(type(target), '__next__'):
            raise TypeError('Weakref proxy referenced a non-iterator')
        return next(target)

    def __contains__(self, item):
        return item in _proxy_target(self)

    def __getitem__(self, key):
        return _proxy_target(self)[key]

    def __setitem__(self, key, value):
        _proxy_target(self)[key] = value

    def __delitem__(self, key):
        del _proxy_target(self)[key]

    def __eq__(self, other):
        return _proxy_target(self) == _proxy_operand(other)

    def __ne__(self, other):
        return _proxy_target(self) != _proxy_operand(other)

    def __lt__(self, other):
        return _proxy_target(self) < _proxy_operand(other)

    def __le__(self, other):
        return _proxy_target(self) <= _proxy_operand(other)

    def __gt__(self, other):
        return _proxy_target(self) > _proxy_operand(other)

    def __ge__(self, other):
        return _proxy_target(self) >= _proxy_operand(other)

    def __add__(self, other):
        return _proxy_target(self) + other

    def __sub__(self, other):
        return _proxy_target(self) - other

    def __mul__(self, other):
        return _proxy_target(self) * other

    def __index__(self):
        return _proxy_target(self).__index__()

    def __int__(self):
        return int(_proxy_target(self))

    def __float__(self):
        return float(_proxy_target(self))

    def __abs__(self):
        return abs(_proxy_target(self))

    def __neg__(self):
        return -_proxy_target(self)

    def __setattr__(self, name, value):
        setattr(_proxy_target(self), name, value)

    def __delattr__(self, name):
        delattr(_proxy_target(self), name)

    def __bytes__(self):
        return bytes(_proxy_target(self))

    def __reversed__(self):
        return reversed(_proxy_target(self))

    def __round__(self, *args):
        return round(_proxy_target(self), *args)

    def __radd__(self, other):
        return other + _proxy_target(self)

    def __rsub__(self, other):
        return other - _proxy_target(self)

    def __rmul__(self, other):
        return other * _proxy_target(self)

    def __matmul__(self, other):
        return _proxy_target(self) @ other

    def __rmatmul__(self, other):
        return other @ _proxy_target(self)

    def __truediv__(self, other):
        return _proxy_target(self) / other

    def __rtruediv__(self, other):
        return other / _proxy_target(self)

    def __floordiv__(self, other):
        return _proxy_target(self) // other

    def __rfloordiv__(self, other):
        return other // _proxy_target(self)

    def __mod__(self, other):
        return _proxy_target(self) % other

    def __rmod__(self, other):
        return other % _proxy_target(self)

    def __pow__(self, other):
        return _proxy_target(self) ** other

    def __rpow__(self, other):
        return other ** _proxy_target(self)

    def __lshift__(self, other):
        return _proxy_target(self) << _proxy_operand(other)

    def __rlshift__(self, other):
        return other << _proxy_target(self)

    def __rshift__(self, other):
        return _proxy_target(self) >> _proxy_operand(other)

    def __rrshift__(self, other):
        return other >> _proxy_target(self)

    def __and__(self, other):
        return _proxy_target(self) & other

    def __rand__(self, other):
        return other & _proxy_target(self)

    def __xor__(self, other):
        return _proxy_target(self) ^ other

    def __rxor__(self, other):
        return other ^ _proxy_target(self)

    def __or__(self, other):
        return _proxy_target(self) | other

    def __ror__(self, other):
        return other | _proxy_target(self)

    def __pos__(self):
        return +_proxy_target(self)

    def __invert__(self):
        return ~_proxy_target(self)

    def __divmod__(self, other):
        return divmod(_proxy_target(self), other)

    def __rdivmod__(self, other):
        return divmod(other, _proxy_target(self))

    def __iadd__(self, other):
        target = _proxy_target(self)
        target += _proxy_operand(other)
        return target

    def __isub__(self, other):
        target = _proxy_target(self)
        target -= _proxy_operand(other)
        return target

    def __imul__(self, other):
        target = _proxy_target(self)
        target *= _proxy_operand(other)
        return target

    def __imatmul__(self, other):
        target = _proxy_target(self)
        target @= _proxy_operand(other)
        return target

    def __itruediv__(self, other):
        target = _proxy_target(self)
        target /= _proxy_operand(other)
        return target

    def __ifloordiv__(self, other):
        target = _proxy_target(self)
        target //= _proxy_operand(other)
        return target

    def __imod__(self, other):
        target = _proxy_target(self)
        target %= _proxy_operand(other)
        return target

    def __ipow__(self, other):
        target = _proxy_target(self)
        target **= _proxy_operand(other)
        return target

    def __ilshift__(self, other):
        target = _proxy_target(self)
        target <<= _proxy_operand(other)
        return target

    def __irshift__(self, other):
        target = _proxy_target(self)
        target >>= _proxy_operand(other)
        return target

    def __iand__(self, other):
        target = _proxy_target(self)
        target &= _proxy_operand(other)
        return target

    def __ixor__(self, other):
        target = _proxy_target(self)
        target ^= _proxy_operand(other)
        return target

    def __ior__(self, other):
        target = _proxy_target(self)
        target |= _proxy_operand(other)
        return target

    __hash__ = None


def _call_proxy(self, *args, **kwargs):
    return _proxy_target(self)(*args, **kwargs)


_callable_proxy_members = dict(ProxyType.__dict__)
for _name in ('__dict__', '__weakref__', '__name__', '__qualname__'):
    _callable_proxy_members.pop(_name, None)
_callable_proxy_members['__call__'] = _call_proxy
_callable_proxy_members['__qualname__'] = 'CallableProxyType'
CallableProxyType = type('CallableProxyType', (object,), _callable_proxy_members)
del _callable_proxy_members


ProxyTypes = (ProxyType, CallableProxyType)


def proxy(*args, **kwargs):
    if kwargs:
        raise TypeError('_weakref.proxy() takes no keyword arguments')
    if not 1 <= len(args) <= 2:
        raise TypeError('proxy expected at ' + ('least 1 argument' if not args else 'most 2 arguments') + ', got ' + str(len(args)))
    ob = args[0]
    callback = args[1] if len(args) == 2 else None
    return _make_weak(ob, CallableProxyType if callable(ob) else ProxyType, callback)


def _proxy_target(value):
    return _get_weak(value, 'proxy')


def _proxy_operand(value):
    if type(value) in ProxyTypes:
        return _proxy_target(value)
    return value


_get_weak(ProxyType, 'intern')
_get_weak(CallableProxyType, 'intern')
_get_weak(ProxyType, 'seal')
_get_weak(CallableProxyType, 'seal')
