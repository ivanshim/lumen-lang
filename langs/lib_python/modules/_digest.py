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
        for i in range(1, len(view._offsets)):
            if view._offsets[i] - view._offsets[i - 1] != view._itemsize:
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

    def update(self, data):
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
