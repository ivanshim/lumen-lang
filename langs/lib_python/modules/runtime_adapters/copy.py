# Runtime adapter for CPython v3.14.8 copy.
# Retains the documented native-object bridges previously mixed into the source.
# The upstream module is initialized first, then these bindings supply runtime protocols.

import pickle as _pickle
import types
import weakref
_MethodType = types.MethodType
d = _deepcopy_dispatch

Error.__module__ = __name__

def copy(x):
    """Shallow copy operation on arbitrary Python objects.

    See the module's __doc__ string for more info.
    """

    cls = type(x)
    if isinstance(x, type):
        return x

    if any(cls is kind for kind in _copy_atomic_types):
        return x
    if cls is bytearray:
        return bytearray(x)
    if any(cls is kind for kind in _copy_builtin_containers):
        return cls.copy(x)


    if isinstance(x, type):
        # treat it as a regular class:
        return x

    copier = getattr(cls, "__copy__", None)
    if copier is not None:
        return copier(x)

    reductor = dispatch_table.get(cls)
    if reductor is not None:
        rv = reductor(x)
    else:
        reductor = _reduction_hook(x, "__reduce_ex__")
        if reductor is not None:
            rv = reductor(4)
        else:
            reductor = _reduction_hook(x, "__reduce__")
            if reductor:
                rv = reductor()
            else:
                rv = _reduce_native(x)

    if isinstance(rv, str):
        return x
    return _reconstruct(x, None, *rv)

_copy_atomic_types = (types.NoneType, int, float, bool, complex, str, tuple,
          bytes, frozenset, type, range, slice, property,
          types.BuiltinFunctionType, types.EllipsisType,
          types.NotImplementedType, types.FunctionType, types.CodeType,
          weakref.ref, super,)

_copy_atomic_types = _copy_atomic_types + (type(copy), type(max), type(Ellipsis), type(NotImplemented), type(property()))

_copy_builtin_containers = (list, dict, set, bytearray,)

def deepcopy(x, memo=None, _nil=[]):
    """Deep copy operation on arbitrary Python objects.

    See the module's __doc__ string for more info.
    """

    cls = type(x)
    if isinstance(x, type):
        return x

    if any(cls is kind for kind in _atomic_types):
        return x

    d = id(x)
    if memo is None:
        memo = {}
    else:
        y = memo.get(d, _nil)
        if y is not _nil:
            return y

    copier = _deepcopy_dispatch.get(cls)
    if copier is not None:
        y = copier(x, memo)
    else:
        if isinstance(x, type):
            y = x # atomic copy
        else:
            copier = getattr(x, "__deepcopy__", None)
            if copier is not None:
                y = copier(memo)
            else:
                reductor = dispatch_table.get(cls)
                if reductor:
                    rv = reductor(x)
                else:
                    reductor = _reduction_hook(x, "__reduce_ex__")
                    if reductor is not None:
                        rv = reductor(4)
                    else:
                        reductor = _reduction_hook(x, "__reduce__")
                        if reductor:
                            rv = reductor()
                        else:
                            rv = _reduce_native(x)
                if isinstance(rv, str):
                    y = x
                else:
                    y = _reconstruct(x, memo, *rv)

    # If is its own copy, don't memoize.
    if y is not x:
        memo[d] = y
        _keep_alive(x, memo) # Make sure x lives at least as long as d
    return y

_atomic_types = (types.NoneType, types.EllipsisType, types.NotImplementedType,
          int, float, bool, complex, bytes, str, types.CodeType, type, range,
          types.BuiltinFunctionType, types.FunctionType, weakref.ref, property,)

_atomic_types = _atomic_types + (type(copy), type(max), type(Ellipsis), type(NotImplemented), type(property()))

def _deepcopy_method(x, memo): # Copy instance methods
    owner = deepcopy(x.__self__, memo)
    function = x.__func__
    bound = getattr(owner, function.__name__, None)
    if getattr(bound, "__func__", None) is function:
        return bound
    return _MethodType(function, owner)

class _MethodProbe:
    def method(self):
        pass

d[type(_MethodProbe().method)] = _deepcopy_method

del _MethodProbe

def replace(obj, /, **changes):
    """Return a new object replacing specified fields with new values.

    This is especially useful for immutable objects, like named tuples or
    frozen dataclasses.
    """
    cls = type(obj)
    func = getattr(cls, '__replace__', None)
    if func is None:
        raise TypeError(f"replace() does not support {cls.__name__} objects")
    return func(obj, **changes)

def _reduce_native(value):
    if type(value) is slice:
        return (slice, (value.start, value.stop, value.step))
    if type(value) in (set, frozenset):
        return (type(value), (list(value),))
    if getattr(value, "__reduce_ex__", None) is None and getattr(value, "__reduce__", None) is None:
        raise Error("un(shallow)copyable object of type %s" % type(value))
    reduction = _pickle._reduce(value, 4)
    if len(reduction) > 2 and not hasattr(value, "__dict__"):
        state = reduction[2]
        if isinstance(state, tuple) and len(state) == 2:
            reduction = (*reduction[:2], (None, state[1]), *reduction[3:])
    return reduction

def _reduction_hook(value, name):
    getattr(value, name, None)
    return _pickle._reduction_hook(value, name)

d[list] = _deepcopy_list

d[tuple] = _deepcopy_tuple

d[dict] = _deepcopy_dict

d[types.MethodType] = _deepcopy_method



def _deepcopy_list(x, memo, deepcopy=deepcopy):
    y = []
    memo[id(x)] = y
    append = y.append
    for a in x:
        append(deepcopy(a, memo))
    return y


def _deepcopy_tuple(x, memo, deepcopy=deepcopy):
    y = [deepcopy(a, memo) for a in x]
    # We're not going to put the tuple in the memo, but it's still important we
    # check for it, in case the tuple contains recursive mutable structures.
    try:
        return memo[id(x)]
    except KeyError:
        pass
    for k, j in zip(x, y):
        if k is not j:
            y = tuple(y)
            break
    else:
        y = x
    return y


def _deepcopy_dict(x, memo, deepcopy=deepcopy):
    y = {}
    memo[id(x)] = y
    for key, value in x.items():
        y[deepcopy(key, memo)] = deepcopy(value, memo)
    return y


def _reconstruct(x, memo, func, args,
                 state=None, listiter=None, dictiter=None,
                 *, deepcopy=deepcopy):
    deep = memo is not None
    if deep and args:
        args = (deepcopy(arg, memo) for arg in args)
    y = func(*args)
    if deep:
        memo[id(x)] = y

    if state is not None:
        if deep:
            state = deepcopy(state, memo)
        if hasattr(y, '__setstate__'):
            y.__setstate__(state)
        else:
            if isinstance(state, tuple) and len(state) == 2:
                state, slotstate = state
            else:
                slotstate = None
            if state is not None:
                y.__dict__.update(state)
            if slotstate is not None:
                for key, value in slotstate.items():
                    setattr(y, key, value)

    if listiter is not None:
        if deep:
            for item in listiter:
                item = deepcopy(item, memo)
                y.append(item)
        else:
            for item in listiter:
                y.append(item)
    if dictiter is not None:
        if deep:
            for key, value in dictiter:
                key = deepcopy(key, memo)
                value = deepcopy(value, memo)
                y[key] = value
        else:
            for key, value in dictiter:
                y[key] = value
    return y


d[list] = _deepcopy_list
d[tuple] = _deepcopy_tuple
d[dict] = _deepcopy_dict
del d, types, weakref
