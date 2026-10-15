# OpenSSL-shaped hashlib backend. The digest algorithms are the builtin
# modules; this module publishes the names CPython's _hashlib module does.
import operator

_native = __crypto

_GIL_MINSIZE = 2048


class UnsupportedDigestmodError(ValueError):
    """A digest constructor this backend does not know."""


def _minted(cls):
    def create(cls, *args, **kwargs):
        if not args or args[0] is not _mint:
            raise TypeError("cannot create '" + cls.__module__ + "." + cls.__name__ + "' instances")
        return object.__new__(cls)
    cls.__new__ = create
    return cls


_mint = object()


@_minted
class HASH:
    """Internal hash object type. Constructed only by this module."""


@_minted
class HASHXOF:
    """Internal extendable-output hash type. Constructed only by this module."""


@_minted
class HMAC:
    """Keyed hash. Constructed only by hmac_new."""

    def __init__(self, token, hash_name, block_size, inner, outer):
        self.name = "hmac-" + hash_name
        self._hash_name = hash_name
        self.block_size = block_size
        self.digest_size = inner.digest_size
        self._inner = inner
        self._outer = outer

    def update(self, msg):
        self._inner.update(_payload(msg))
        return None

    def copy(self):
        twin = HMAC(_mint, self._hash_name, self.block_size, self._inner.copy(), self._outer.copy())
        return twin

    def digest(self):
        outer = self._outer.copy()
        outer.update(self._inner.digest())
        return outer.digest()

    def hexdigest(self):
        return self.digest().hex()

    def __repr__(self):
        return "<" + self._hash_name + " HMAC object @" + hex(id(self)) + ">"


def _payload(data):
    if isinstance(data, str):
        raise TypeError("a bytes-like object is required, not 'str'")
    if isinstance(data, (bytes, bytearray)):
        return bytes(data)
    if isinstance(data, memoryview):
        return data.tobytes()
    try:
        view = memoryview(data)
    except TypeError:
        raise TypeError("a bytes-like object is required, not '" + type(data).__name__ + "'") from None
    return view.tobytes()


def _prepare_compare(value):
    if isinstance(value, str):
        try:
            return value.encode("ascii"), "text"
        except UnicodeEncodeError:
            raise TypeError("comparing strings with non-ASCII characters is not supported") from None
    if isinstance(value, (bytes, bytearray, memoryview)):
        return _payload(value), "bytes"
    raise TypeError("a bytes-like object is required, not '" + type(value).__name__ + "'")


class _CompareDigest:
    """Callable that does not bind when stored on a class, matching a builtin."""

    def __call__(self, a, b):
        left, left_kind = _prepare_compare(a)
        try:
            right, right_kind = _prepare_compare(b)
        except TypeError:
            if left_kind == "text" and not isinstance(b, str):
                raise TypeError("a bytes-like object is required, not 'str'") from None
            raise
        if left_kind != right_kind:
            if left_kind == "text":
                raise TypeError("a bytes-like object is required, not 'str'")
            raise TypeError("a bytes-like object is required, not '" + type(b).__name__ + "'")
        if len(left) != len(right):
            return False
        return _native(2, left, right)


compare_digest = _CompareDigest()


def _known():
    from _md5 import md5
    from _sha1 import sha1
    from _sha2 import sha224, sha256, sha384, sha512
    from _sha3 import sha3_224, sha3_256, sha3_384, sha3_512, shake_128, shake_256
    return (
        ("md5", md5),
        ("sha1", sha1),
        ("sha224", sha224),
        ("sha256", sha256),
        ("sha384", sha384),
        ("sha512", sha512),
        ("sha3_224", sha3_224),
        ("sha3_256", sha3_256),
        ("sha3_384", sha3_384),
        ("sha3_512", sha3_512),
        ("shake_128", shake_128),
        ("shake_256", shake_256),
    )


def _tables():
    by_name = {}
    by_ctor = {}
    for name, constructor in _known():
        by_name[name] = constructor
        by_ctor[constructor] = name
    return by_name, by_ctor


def _resolve(digestmod):
    by_name, by_ctor = _tables()
    if isinstance(digestmod, str):
        constructor = by_name.get(digestmod.lower())
        if constructor is None:
            raise UnsupportedDigestmodError("unsupported hash type " + digestmod)
        return digestmod.lower(), constructor
    name = by_ctor.get(digestmod)
    if name is None:
        raise UnsupportedDigestmodError("unsupported hash type")
    return name, digestmod


def _hmac_parts(key, digestmod):
    name, constructor = _resolve(digestmod)
    key = _payload(key)
    probe = constructor()
    block = probe.block_size
    if len(key) > block:
        key = constructor(key).digest()
    if len(key) < block:
        key = key + b"\x00" * (block - len(key))
    inner_pad = bytes(byte ^ 0x36 for byte in key)
    outer_pad = bytes(byte ^ 0x5C for byte in key)
    inner = constructor(inner_pad)
    outer = constructor(outer_pad)
    return name, block, inner, outer


def hmac_new(key, msg=None, *, digestmod):
    name, block, inner, outer = _hmac_parts(key, digestmod)
    obj = HMAC(_mint, name, block, inner, outer)
    if msg is not None:
        obj.update(msg)
    return obj


def hmac_digest(key, msg, digest):
    obj = hmac_new(key, msg, digestmod=digest)
    return obj.digest()


def new(name, *args, **kwargs):
    if not isinstance(name, str):
        raise TypeError("new() argument 'name' must be str, not " + type(name).__name__)
    by_name, _by_ctor = _tables()
    constructor = by_name.get(name.lower())
    if constructor is None:
        raise ValueError("unsupported hash type " + name)
    return constructor(*args, **kwargs)


def pbkdf2_hmac(hash_name, password, salt, iterations, dklen=None):
    if not isinstance(hash_name, str):
        raise TypeError("pbkdf2_hmac() argument 'hash_name' must be str, not " + type(hash_name).__name__)
    password = _payload(password)
    salt = _payload(salt)
    iterations = operator.index(iterations)
    if iterations < 1:
        raise ValueError("iteration value must be greater than 0")
    try:
        probe = new(hash_name)
    except ValueError:
        raise UnsupportedDigestmodError("unsupported hash type " + hash_name) from None
    block = probe.digest_size
    if block < 1:
        raise ValueError("unsupported hash type " + hash_name)
    if dklen is None:
        dklen = block
    else:
        dklen = operator.index(dklen)
        if dklen < 1:
            raise ValueError("key length must be greater than 0")
    return _native(6, hash_name.lower(), password, salt, iterations, dklen)


def scrypt(password, *, salt, n, r, p, maxmem=0, dklen=64):
    password = _payload(password)
    salt = _payload(salt)
    if n is None or r is None or p is None or dklen is None or maxmem is None:
        raise TypeError("an integer is required")
    n = operator.index(n)
    r = operator.index(r)
    p = operator.index(p)
    maxmem = operator.index(maxmem)
    dklen = operator.index(dklen)
    if n <= 1 or n & (n - 1):
        raise ValueError("n must be a power of 2")
    if r < 1 or p < 1:
        raise ValueError("Invalid parameter combination for n, r, p, maxmem.")
    if dklen < 1 or dklen >= 2147483647:
        raise ValueError("dklen must be greater than 0 and smaller than 2147483647")
    if maxmem < 0:
        raise ValueError("maxmem must be positive")
    if maxmem == 0:
        maxmem = 32 * 1024 * 1024
    needed = 128 * r * n
    if needed > maxmem:
        raise ValueError("memory limit exceeded")
    return _native(5, password, salt, n, r, p, dklen)


def get_fips_mode():
    return 0


_by_name, _by_ctor = _tables()
openssl_md5 = _by_name["md5"]
openssl_sha1 = _by_name["sha1"]
openssl_sha224 = _by_name["sha224"]
openssl_sha256 = _by_name["sha256"]
openssl_sha384 = _by_name["sha384"]
openssl_sha512 = _by_name["sha512"]
openssl_sha3_224 = _by_name["sha3_224"]
openssl_sha3_256 = _by_name["sha3_256"]
openssl_sha3_384 = _by_name["sha3_384"]
openssl_sha3_512 = _by_name["sha3_512"]
openssl_shake_128 = _by_name["shake_128"]
openssl_shake_256 = _by_name["shake_256"]
openssl_md_meth_names = frozenset(_by_name)
_constructors = {constructor: name for name, constructor in _by_name.items()}

for _cls in (HASH, HASHXOF, HMAC):
    _cls.__module__ = "_hashlib"
    _native(4, _cls)
del _cls
