# A new thing with named fields changed, after the thing's own
# __replace__ the way the reference spells it.
def replace(obj, /, **changes):
    cls = obj.__class__
    func = getattr(cls, '__replace__', None)
    if func is None:
        raise TypeError('replace() does not support ' + cls.__name__ + ' objects')
    return func(obj, **changes)

# Copying uses the same reconstruction and state hooks as pickling.
import pickle


class Error(Exception):
    pass


def _atomic(value):
    return value is None or value is NotImplemented or value is Ellipsis or type(value) in (bool, int, float, complex, str, bytes, range, type) or type(value) in (type(_atomic), type(iter))


def _reconstruct(value, reduction, memo=None):
    if isinstance(reduction, str):
        return value
    constructor, args = reduction[:2]
    if memo is not None:
        args = deepcopy(args, memo)
    result = constructor(*args)
    if memo is not None:
        memo[id(value)] = result
    state = reduction[2] if len(reduction) > 2 else None
    if memo is not None:
        state = deepcopy(state, memo)
    pickle._apply_state(result, state)
    if len(reduction) > 3 and reduction[3] is not None:
        for item in reduction[3]:
            result.append(deepcopy(item, memo) if memo is not None else item)
    if len(reduction) > 4 and reduction[4] is not None:
        for key, item in reduction[4]:
            if memo is not None:
                key = deepcopy(key, memo)
                item = deepcopy(item, memo)
            result[key] = item
    return result


def copy(value):
    if _atomic(value):
        return value
    method = getattr(type(value), '__copy__', None)
    if method is not None:
        return method(value)
    cls = type(value)
    if cls in (list, dict, set, bytearray):
        return value.copy() if cls is not bytearray else bytearray(value)
    if cls in (tuple, frozenset, slice):
        return value
    return _reconstruct(value, pickle._reduce(value, 4))


def deepcopy(value, memo=None):
    if memo is None:
        memo = {}
    if _atomic(value):
        return value
    where = id(value)
    if where in memo:
        return memo[where]
    method = getattr(value, '__deepcopy__', None)
    if method is not None:
        result = method(memo)
        memo[where] = result
        return result
    cls = type(value)
    if cls is list:
        result = []
        memo[where] = result
        for item in value:
            result.append(deepcopy(item, memo))
    elif cls is dict:
        result = {}
        memo[where] = result
        for key, item in value.items():
            result[deepcopy(key, memo)] = deepcopy(item, memo)
    elif cls is tuple:
        items = [deepcopy(item, memo) for item in value]
        if where in memo:
            return memo[where]
        result = tuple(items)
        if all(old is new for old, new in zip(value, items)):
            result = value
    elif cls in (set, frozenset):
        result = cls(deepcopy(item, memo) for item in value)
    else:
        return _reconstruct(value, pickle._reduce(value, 4), memo)
    memo[where] = result
    return result
