# Interface for CPython v3.14.8 Modules/hmacmodule.c.
from _digest import _buffer, _SIZES, hmac_class, BuiltinFunction

_GIL_MINSIZE = 2048


class UnknownHashError(ValueError):
    pass


HMAC = hmac_class('_hmac')
_missing = object()


def _name(digestmod):
    name = digestmod.lower() if isinstance(digestmod, str) else None
    if name not in _SIZES:
        raise UnknownHashError('unsupported hash type: ' + repr(digestmod))
    return name


def new(key, msg=None, digestmod=_missing):
    key = _buffer(key)
    msg = b'' if msg is None else _buffer(msg)
    if digestmod is _missing:
        raise TypeError("new() missing 1 required argument 'digestmod'")
    return HMAC._make(_name(digestmod), key, msg)


def compute_digest(key, msg, digest):
    key = _buffer(key)
    msg = _buffer(msg)
    return HMAC._make(_name(digest), key, msg).digest()


def _one_shot(algorithm):
    return lambda key, msg: compute_digest(key, msg, algorithm)


for _algorithm in _SIZES:
    _label = 'compute_' + _algorithm + ('_32' if _algorithm.startswith('blake2') else '')
    globals()[_label] = BuiltinFunction('_hmac', _label, _one_shot(_algorithm))
del _algorithm, _label

__crypto(4, HMAC)
