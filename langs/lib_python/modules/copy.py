# Objects may give their own account of copying.
class Error(Exception):
    pass


def copy(value):
    return __copy_value(value, False)


def deepcopy(value, memo=None):
    if memo is not None:
        raise 'NotImplementedError: an external deepcopy memo needs shared mapping storage'
    return __copy_value(value, True)


def replace(obj, /, **changes):
    method = getattr(obj, '__replace__', None)
    if method is None:
        raise 'TypeError: replace does not support this object'
    return method(**changes)
