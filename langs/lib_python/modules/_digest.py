# Python object interface for the v3.14.8 builtin digest modules.
# Hash computation is supplied independently by each full kernel.
import operator
_native_digest = __crypto


def _buffer(data):
    if isinstance(data, str):
        raise TypeError('Strings must be encoded before hashing')
    if isinstance(data, (bytes, bytearray)):
        return bytes(data)
    import array
    if isinstance(data, array.array):
        return data.tobytes()
    try:
        view = memoryview(data)
        view._check()
        if not view.c_contiguous:
            raise BufferError('memoryview: underlying buffer is not C-contiguous')
        return view.tobytes()
    except TypeError:
        raise TypeError('object supporting the buffer API required') from None


def _c_int(value):
    value = operator.index(value)
    if value < -2147483648 or value > 2147483647:
        raise OverflowError('Python int too large to convert to C int')
    return value


def _parameter_buffer(value):
    if isinstance(value, str):
        raise TypeError("a bytes-like object is required, not 'str'")
    try:
        return _buffer(value)
    except TypeError:
        raise TypeError("a bytes-like object is required, not '" + type(value).__name__ + "'") from None


def _arguments(name, args, kwargs, defaults):
    if len(args) > 1:
        raise TypeError(name + '() takes at most 1 positional argument (' + str(len(args)) + ' given)')
    if args and 'data' in kwargs:
        raise TypeError("argument for " + name + "() given by name ('data') and position (1)")
    for key in kwargs:
        if key not in defaults and key not in ('data', 'string'):
            raise TypeError(name + "() got an unexpected keyword argument '" + key + "'")
    if 'string' in kwargs and (args or 'data' in kwargs):
        raise TypeError("'data' and 'string' are mutually exclusive and support for 'string' keyword parameter is slated for removal in a future version.")
    data = args[0] if args else kwargs.get('data', kwargs.get('string', b''))
    options = defaults.copy()
    for key in kwargs:
        if key in options:
            options[key] = kwargs[key]
    return data, options


class Hash:
    def __init__(self, *args, **kwargs):
        data, options = _arguments(self._algorithm, args, kwargs, {'usedforsecurity': True})
        bool(options['usedforsecurity'])
        self._data = _buffer(data)
        self._size = self._digest_size
        self._key = self._salt = self._person = b''
        self._tree = (1, 1, 0, 0, 0, 0, 0)

    @property
    def name(self):
        return self._algorithm

    @property
    def digest_size(self):
        return self._size

    @property
    def block_size(self):
        return self._block_size

    def update(self, data, /):
        self._data += _buffer(data)

    def copy(self):
        other = self.__class__.__new__(self.__class__)
        other._data = self._data
        other._size = self._size
        other._key = self._key
        other._salt = self._salt
        other._person = self._person
        other._tree = self._tree
        return other

    def digest(self):
        return _native_digest(0, self.name, self._data, self._size,
                              self._key, self._salt, self._person, *self._tree)

    def hexdigest(self):
        return self.digest().hex()


class Shake(Hash):
    def digest(self, length):
        length = operator.index(length)
        if length < 0:
            raise ValueError('Cannot convert negative int')
        if length > 18446744073709551615:
            raise OverflowError('Python int too large for C unsigned long')
        if length >= 536870912:
            raise ValueError('length is too large')
        return _native_digest(0, self.name, self._data, length,
                              b'', b'', b'', *self._tree)

    def hexdigest(self, length):
        return self.digest(length).hex()


class Blake(Hash):
    def __init__(self, *args, **kwargs):
        data, options = _arguments(self._algorithm, args, kwargs, {
            'digest_size': self.MAX_DIGEST_SIZE, 'key': b'', 'salt': b'',
            'person': b'', 'fanout': 1, 'depth': 1, 'leaf_size': 0,
            'node_offset': 0, 'node_depth': 0, 'inner_size': 0,
            'last_node': False, 'usedforsecurity': True})
        digest_size = options['digest_size']
        key, salt, person = options['key'], options['salt'], options['person']
        fanout, depth = options['fanout'], options['depth']
        leaf_size, node_offset = options['leaf_size'], options['node_offset']
        node_depth, inner_size = options['node_depth'], options['inner_size']
        last_node = options['last_node']
        digest_size = _c_int(digest_size)
        key, salt, person = _parameter_buffer(key), _parameter_buffer(salt), _parameter_buffer(person)
        fanout, depth = _c_int(fanout), _c_int(depth)
        leaf_size, node_offset = operator.index(leaf_size), operator.index(node_offset)
        node_depth, inner_size = _c_int(node_depth), _c_int(inner_size)
        if leaf_size < 0 or node_offset < 0:
            raise ValueError('Cannot convert negative int')
        if leaf_size > 18446744073709551615:
            raise OverflowError('Python int too large for C unsigned long')
        if node_offset > 18446744073709551615:
            raise OverflowError('Python int too large for C unsigned long long')
        last_node = bool(last_node)
        bool(options['usedforsecurity'])
        if not 1 <= digest_size <= self.MAX_DIGEST_SIZE:
            raise ValueError('digest_size for ' + ('Blake2b' if self._algorithm == 'blake2b' else 'Blake2s') + ' must be between 1 and ' + str(self.MAX_DIGEST_SIZE) + ' bytes, here it is ' + str(digest_size))
        if len(key) > self.MAX_KEY_SIZE:
            raise ValueError('maximum key length is ' + str(self.MAX_KEY_SIZE) + ' bytes')
        if len(salt) > self.SALT_SIZE:
            raise ValueError('maximum salt length is ' + str(self.SALT_SIZE) + ' bytes')
        if len(person) > self.PERSON_SIZE:
            raise ValueError('maximum person length is ' + str(self.PERSON_SIZE) + ' bytes')
        if not 0 <= fanout <= 255:
            raise ValueError('fanout must be between 0 and 255')
        if not 1 <= depth <= 255:
            raise ValueError('depth must be between 1 and 255')
        if not 0 <= node_depth <= 255:
            raise ValueError('node_depth must be between 0 and 255')
        if not 0 <= inner_size <= self.MAX_DIGEST_SIZE:
            raise ValueError('inner_size must be between 0 and is ' + str(self.MAX_DIGEST_SIZE))
        if leaf_size < 0:
            raise ValueError('Cannot convert negative int')
        if node_offset < 0:
            raise ValueError('Cannot convert negative int')
        if leaf_size > 4294967295:
            raise OverflowError('leaf_size is too large')
        if node_offset > self._max_offset:
            raise OverflowError('node_offset is too large')
        self._data = _buffer(data)
        self._size = digest_size
        self._key, self._salt, self._person = key, salt, person
        self._tree = (fanout, depth, leaf_size, node_offset, node_depth, inner_size, int(bool(last_node)))


# Sizes shared by the OpenSSL-style (_hashlib) and HACL*-style (_hmac) modules.
_SIZES = {
    'md5': (16, 64), 'sha1': (20, 64), 'sha224': (28, 64), 'sha256': (32, 64),
    'sha384': (48, 128), 'sha512': (64, 128), 'sha3_224': (28, 144),
    'sha3_256': (32, 136), 'sha3_384': (48, 104), 'sha3_512': (64, 72),
    'blake2b': (64, 128), 'blake2s': (32, 64)}
_XOF_BLOCKS = {'shake_128': 168, 'shake_256': 136}


def one_shot(name, data, size):
    return _native_digest(0, name, data, size, b'', b'', b'', 1, 1, 0, 0, 0, 0, 0)


_TRANS_36 = bytes((x ^ 0x36) for x in range(256))
_TRANS_5C = bytes((x ^ 0x5C) for x in range(256))


class BuiltinFunction:
    """Callable with the non-binding behavior of a C builtin function."""
    def __init__(self, module, name, call, doc=None):
        self.__module__ = module
        self.__name__ = self.__qualname__ = name
        self.__doc__ = doc
        self._call = call

    def __call__(self, *args, **kwargs):
        return self._call(*args, **kwargs)

    def __repr__(self):
        return '<built-in function ' + self.__name__ + '>'


def hmac_class(module):
    """Build the sealed keyed-hash type shared by both HMAC front ends."""
    def make(cls, name, key, data):
        self = object.__new__(cls)
        size, block = _SIZES[name]
        if len(key) > block:
            key = one_shot(name, key, size)
        key = key.ljust(block, b'\0')
        self._name = name
        self._pads = (key.translate(_TRANS_36), key.translate(_TRANS_5C))
        self._data = data
        return self

    def new(cls, *args, **kwargs):
        raise TypeError("cannot create '" + module + ".HMAC' instances")

    def update(self, msg):
        self._data += _buffer(msg)

    def copy(self):
        other = object.__new__(type(self))
        other._name, other._pads, other._data = self._name, self._pads, self._data
        return other

    def digest(self):
        size = _SIZES[self._name][0]
        inner = one_shot(self._name, self._pads[0] + self._data, size)
        return one_shot(self._name, self._pads[1] + inner, size)

    def hexdigest(self):
        return self.digest().hex()

    def repr_(self):
        return '<' + self._name + ' HMAC object @ ' + hex(id(self)) + '>'

    space = {
        '__module__': module, '__slots__': ('_name', '_pads', '_data'),
        '__new__': new, '__repr__': repr_, 'update': update, 'copy': copy,
        'digest': digest, 'hexdigest': hexdigest,
        'name': property(lambda self: 'hmac-' + self._name),
        'digest_size': property(lambda self: _SIZES[self._name][0]),
        'block_size': property(lambda self: _SIZES[self._name][1])}
    cls = type('HMAC', (), space)
    cls._make = staticmethod(lambda name, key, data: make(cls, name, key, data))
    return cls
