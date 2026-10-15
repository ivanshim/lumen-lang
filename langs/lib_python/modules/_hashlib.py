# Interface for CPython v3.14.8 Modules/_hashopenssl.c.
import operator
from _digest import (_buffer, _arguments, _parameter_buffer, _SIZES, _XOF_BLOCKS,
                     one_shot, hmac_class, BuiltinFunction)
from _operator import _compare_digest as _compare

_GIL_MINSIZE = 2048
_native_digest = __crypto


class UnsupportedDigestmodError(ValueError):
    pass


def _canonical(name):
    key = name.lower()
    if key.startswith('sha-'):
        key = 'sha' + key[4:]
    elif key.startswith(('sha3-', 'shake-')):
        key = key.replace('-', '_')
    return key if key in _SIZES or key in _XOF_BLOCKS else None


class HASH:
    __slots__ = ('_name', '_data')

    def __new__(cls, *args, **kwargs):
        raise TypeError("cannot create '" + cls.__module__ + '.' + cls.__name__ + "' instances")

    @property
    def name(self):
        return self._name

    @property
    def digest_size(self):
        return _SIZES[self._name][0] if self._name in _SIZES else 0

    @property
    def block_size(self):
        return _SIZES[self._name][1] if self._name in _SIZES else _XOF_BLOCKS[self._name]

    def update(self, obj, /):
        self._data += _buffer(obj)

    def copy(self):
        other = object.__new__(type(self))
        other._name, other._data = self._name, self._data
        return other

    def digest(self):
        return one_shot(self._name, self._data, self.digest_size)

    def hexdigest(self):
        return self.digest().hex()

    def __repr__(self):
        return '<' + self._name + ' ' + type(self).__module__ + '.' + type(self).__name__ + ' object @ ' + hex(id(self)) + '>'


class HASHXOF(HASH):
    def digest(self, length):
        length = operator.index(length)
        if length < 0:
            raise SystemError('Negative size passed to PyBytes_FromStringAndSize')
        if length >= 536870912:
            raise ValueError('length is too large')
        return one_shot(self._name, self._data, length)

    def hexdigest(self, length):
        return self.digest(length).hex()


def _construct(name, label, args, kwargs):
    data, options = _arguments(label, args, kwargs, {'usedforsecurity': True})
    bool(options['usedforsecurity'])
    canonical = _canonical(name)
    if canonical is None:
        raise UnsupportedDigestmodError('[digital envelope routines] unsupported')
    self = object.__new__(HASHXOF if canonical in _XOF_BLOCKS else HASH)
    self._name = canonical
    self._data = _buffer(data)
    return self


def new(name, *args, **kwargs):
    if not isinstance(name, str):
        raise TypeError("new() argument 'name' must be str, not " + type(name).__name__)
    return _construct(name, 'new', args, kwargs)


def _constructor(algorithm):
    label = 'openssl_' + algorithm
    return BuiltinFunction('_hashlib', label,
                           lambda *args, **kwargs: _construct(algorithm, label, args, kwargs),
                           'Returns a ' + algorithm + ' hash object; optionally initialized with a string')


_constructors = {}
for _algorithm in ('md5', 'sha1', 'sha224', 'sha256', 'sha384', 'sha512', 'sha3_224',
                   'sha3_256', 'sha3_384', 'sha3_512', 'shake_128', 'shake_256'):
    _function = _constructor(_algorithm)
    globals()['openssl_' + _algorithm] = _function
    _constructors[_function] = _algorithm
del _algorithm, _function

openssl_md_meth_names = frozenset(_SIZES) | frozenset(_XOF_BLOCKS)

HMAC = hmac_class('_hashlib')
_missing = object()


def _digestmod_name(digestmod):
    if isinstance(digestmod, str):
        name = _canonical(digestmod)
        if name is None:
            raise UnsupportedDigestmodError('[digital envelope routines] unsupported')
    else:
        name = _constructors.get(digestmod)
        if name is None:
            raise UnsupportedDigestmodError('Unsupported digestmod ' + repr(digestmod))
    if name in _XOF_BLOCKS:
        raise ValueError('no reason supplied')
    return name


def hmac_new(key, msg=b'', digestmod=_missing):
    key = _parameter_buffer(key)
    msg = b'' if msg is None else _buffer(msg)
    if digestmod is _missing:
        raise TypeError("Missing required parameter 'digestmod'.")
    return HMAC._make(_digestmod_name(digestmod), key, msg)


def hmac_digest(key, msg, digest):
    key = _parameter_buffer(key)
    msg = _buffer(msg)
    return HMAC._make(_digestmod_name(digest), key, msg).digest()


compare_digest = BuiltinFunction('_hashlib', 'compare_digest', lambda a, b, /: _compare(a, b))


def pbkdf2_hmac(hash_name, password, salt, iterations, dklen=None):
    if not isinstance(hash_name, str):
        raise TypeError("pbkdf2_hmac() argument 'hash_name' must be str, not " + type(hash_name).__name__)
    name = _digestmod_name(hash_name)
    password, salt = _parameter_buffer(password), _parameter_buffer(salt)
    iterations = operator.index(iterations)
    if iterations > 2147483647:
        raise OverflowError('iteration value is too great.')
    if iterations < 1:
        raise ValueError('iteration value must be greater than 0.')
    size = _SIZES[name][0]
    if dklen is None:
        dklen = size
    dklen = operator.index(dklen)
    if dklen > 2147483647:
        raise OverflowError('key length is too great.')
    if dklen < 1:
        raise ValueError('key length must be greater than 0.')
    ipad, opad = HMAC._make(name, password, b'')._pads
    out = b''
    for block in range(1, (dklen + size - 1) // size + 1):
        last = one_shot(name, opad + one_shot(name, ipad + salt + block.to_bytes(4, 'big'), size), size)
        total = int.from_bytes(last, 'big')
        for _ in range(iterations - 1):
            last = one_shot(name, opad + one_shot(name, ipad + last, size), size)
            total ^= int.from_bytes(last, 'big')
        out += total.to_bytes(size, 'big')
    return out[:dklen]


def get_fips_mode():
    return 0


__crypto(4, HASH)
__crypto(4, HASHXOF)
__crypto(4, HMAC)
