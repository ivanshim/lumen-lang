# Array semantics follow CPython v3.14.8 Modules/arraymodule.c (PSF License).
import operator
import struct
import sys

__all__ = ['array', 'ArrayType', 'typecodes']
typecodes = 'bBuhHiIlLqQfdw'
_missing = object()
_native = __struct_native
_type_sizes = {code: _native(0, "@" + ("I" if code in "uw" else code))[0] for code in typecodes}
def _buffer_info(buffer):
    return _native(3, '', buffer)

class array:
    __slots__ = ('_typecode', '_itemsize', '_buffer', '__weakref__')
    def __new__(cls, typecode, initializer=_missing, /, **kwargs):
        if cls is array and kwargs:
            raise TypeError('array.array() takes no keyword arguments')
        if not isinstance(typecode, str) or len(typecode) != 1:
            raise TypeError('array() argument 1 must be a unicode character, not ' + type(typecode).__name__)
        if typecode not in typecodes:
            raise ValueError('bad typecode (must be b, B, u, h, H, i, I, l, L, q, Q, f, d, or w)')
        self = object.__new__(cls)
        self._typecode = typecode
        self._itemsize = _type_sizes[typecode]
        self._buffer = bytearray()
        if typecode == 'u':
            import warnings
            warnings.warn("The 'u' type code is deprecated and will be removed in Python 3.16", DeprecationWarning, stacklevel=2)
        if initializer is not _missing:
            if isinstance(initializer, (bytes, bytearray, memoryview)):
                self.frombytes(initializer)
            elif isinstance(initializer, str):
                if typecode not in 'uw':
                    raise TypeError("cannot use a str to initialize an array with typecode '" + typecode + "'")
                self.fromunicode(initializer)
            elif isinstance(initializer, array) and initializer.typecode in 'uw' and typecode not in 'uw':
                raise TypeError('cannot use a unicode array to initialize an array with typecode ' + repr(typecode))
            elif isinstance(initializer, array):
                self.fromlist(initializer.tolist())
            elif isinstance(initializer, list):
                self.fromlist(initializer)
            else:
                self.extend(initializer)
        return self

    def __init__(self, *args, **kwargs):
        pass

    @property
    def typecode(self):
        return self._typecode

    @property
    def itemsize(self):
        return self._itemsize

    @property
    def data(self):
        return self.tolist()

    def _encode(self, value):
        if self._typecode in 'uw':
            if not isinstance(value, str) or len(value) != 1:
                raise TypeError('array item must be unicode character')
            return _native(1, '@I', (ord(value),))
        if self._typecode in 'fd':
            if isinstance(value, (str, bytes, bytearray)):
                raise TypeError('must be real number, not ' + type(value).__name__)
            value = float(value)
        else:
            value = operator.index(value)
        try:
            return _native(1, '@' + self._typecode, (value,))
        except ValueError:
            raise OverflowError('array item is out of range')
        except OverflowError:
            if self._typecode == 'f':
                return _native(1, '@f', (float('-inf') if value < 0 else float('inf'),))
            raise

    def _decode(self, raw):
        if self._typecode in 'uw':
            return chr(_native(2, '@I', bytes(raw))[0])
        return _native(2, '@' + self._typecode, bytes(raw))[0]

    def _resize_check(self, byte_length):
        if byte_length != len(self._buffer) and _native(6, '', self._buffer):
            raise BufferError('cannot resize an array that is exporting buffers')

    def _extend_raw(self, raw):
        self._resize_check(len(self._buffer) + len(raw))
        self._buffer.extend(raw)

    def __len__(self):
        return len(self._buffer) // self._itemsize

    def __iter__(self):
        return _ArrayIterator(self)

    def __getitem__(self, key):
        if isinstance(key, slice):
            start, stop, step = key.indices(len(self))
            code = 'I' if self._typecode in 'uw' else self._typecode
            raw = _native(9, '@' + code, (self._buffer, start, stop, step))
            return array(self._typecode, raw)
        index = operator.index(key)
        if index < 0:
            index += len(self)
        if index < 0 or index >= len(self):
            raise IndexError('array index out of range')
        buffer = self._buffer
        return self._decode(buffer[index * self._itemsize:(index + 1) * self._itemsize])

    def __setitem__(self, key, value):
        buffer = self._buffer
        if isinstance(key, slice):
            if not isinstance(value, array):
                raise TypeError('can only assign array (not "' + type(value).__name__ + '") to array slice')
            if value.typecode != self._typecode:
                raise TypeError('bad argument type for built-in operation')
            start, stop, step = key.indices(len(self))
            if step == 1:
                raw = value.tobytes()
                self._resize_check(len(buffer) - (max(start, stop) - start) * self._itemsize + len(raw))
                buffer[start * self._itemsize:max(start, stop) * self._itemsize] = raw
            else:
                indices = list(range(start, stop, step))
                if len(indices) != len(value):
                    raise ValueError('attempt to assign array of size ' + str(len(value)) + ' to extended slice of size ' + str(len(indices)))
                raw = value.tobytes()
                for i, index in enumerate(indices):
                    buffer[index * self._itemsize:(index + 1) * self._itemsize] = raw[i * self._itemsize:(i + 1) * self._itemsize]
            return
        index = operator.index(key)
        if index < 0:
            index += len(self)
        if index < 0 or index >= len(self):
            raise IndexError('array assignment index out of range')
        raw = self._encode(value)
        if index >= len(self):
            raise IndexError('array assignment index out of range')
        buffer[index * self._itemsize:(index + 1) * self._itemsize] = raw

    def __delitem__(self, key):
        buffer = self._buffer
        if isinstance(key, slice):
            start, stop, step = key.indices(len(self))
            self._resize_check(len(buffer) - len(range(start, stop, step)) * self._itemsize)
            if step == 1:
                buffer[start * self._itemsize:max(start, stop) * self._itemsize] = b''
            else:
                for index in sorted(range(start, stop, step), reverse=True):
                    buffer[index * self._itemsize:(index + 1) * self._itemsize] = b''
            return
        index = operator.index(key)
        if index < 0:
            index += len(self)
        if index < 0 or index >= len(self):
            raise IndexError('array assignment index out of range')
        self._resize_check(len(buffer) - self._itemsize)
        buffer[index * self._itemsize:(index + 1) * self._itemsize] = b''

    def append(self, value):
        self._extend_raw(self._encode(value))

    def extend(self, values):
        if isinstance(values, array):
            if values.typecode != self._typecode:
                raise TypeError('can only extend with array of same kind')
            self._extend_raw(values.tobytes())
        else:
            for value in values:
                self.append(value)

    def insert(self, index, value):
        buffer = self._buffer
        index = operator.index(index)
        if index < 0:
            index = max(0, index + len(self))
        index = min(index, len(self))
        raw = self._encode(value)
        self._resize_check(len(buffer) + len(raw))
        buffer[index * self._itemsize:index * self._itemsize] = raw

    def pop(self, index=-1):
        if not len(self):
            raise IndexError('pop from empty array')
        try:
            result = self[index]
        except IndexError:
            raise IndexError('pop index out of range')
        del self[index]
        return result

    def clear(self):
        self._resize_check(0)
        self._buffer.clear()

    def reverse(self):
        buffer = self._buffer
        code = 'I' if self._typecode in 'uw' else self._typecode
        buffer[:] = _native(7, '@' + code, buffer)

    def count(self, value):
        if type(value) in (int, float, str):
            return self.tolist().count(value)
        return sum(1 for item in self if item == value)

    def index(self, value, start=0, stop=sys.maxsize):
        start, stop, step = slice(start, stop, 1).indices(len(self))
        if type(value) in (int, float, str):
            try:
                return self.tolist().index(value, start, stop)
            except ValueError:
                raise ValueError('array.index(x): x not in array')
        for i in range(start, stop):
            if self[i] == value:
                return i
        raise ValueError('array.index(x): x not in array')

    def remove(self, value):
        try:
            index = self.index(value)
        except ValueError:
            raise ValueError('array.remove(x): x not in array')
        del self[index]

    def tolist(self):
        if self._typecode in 'uw':
            return [chr(n) for n in _native(2, '@' + str(len(self)) + 'I', self.tobytes())]
        return list(_native(2, '@' + str(len(self)) + self._typecode, self.tobytes()))

    def fromlist(self, values):
        if not isinstance(values, list):
            raise TypeError('arg must be list')
        if self._typecode not in 'uw':
            try:
                raw = _native(1, '@' + str(len(values)) + self._typecode, tuple(values))
            except (ValueError, OverflowError):
                raw = b''.join(self._encode(item) for item in values)
        else:
            raw = b''.join(self._encode(item) for item in values)
        self._extend_raw(raw)

    def tobytes(self):
        return bytes(self._buffer)

    def __bytes__(self):
        return self.tobytes()

    def frombytes(self, values):
        if isinstance(values, (bytes, bytearray)):
            raw = bytes(values)
            if len(raw) % self._itemsize:
                raise ValueError('bytes length not a multiple of item size')
            self._extend_raw(raw)
            return
        view = memoryview(values)
        try:
            try:
                byteview = view.cast('B')
            except TypeError:
                raise BufferError('memoryview: underlying buffer is not C-contiguous')
            try:
                if view.itemsize != 1:
                    raise TypeError('a bytes-like object is required')
                raw = byteview.tobytes()
                if len(raw) % self._itemsize:
                    raise ValueError('bytes length not a multiple of item size')
                self._extend_raw(raw)
            finally:
                byteview.release()
        finally:
            view.release()

    def byteswap(self):
        buffer = self._buffer
        code = 'I' if self._typecode in 'uw' else self._typecode
        buffer[:] = _native(8, '@' + code, buffer)

    def fromunicode(self, values):
        if self._typecode not in 'uw':
            raise ValueError('fromunicode() may only be called on unicode type arrays')
        if not isinstance(values, str):
            raise TypeError('fromunicode() argument must be str')
        raw = _native(1, '@' + str(len(values)) + 'I', tuple(map(ord, values)))
        self._extend_raw(raw)

    def tounicode(self):
        if self._typecode not in 'uw':
            raise ValueError('tounicode() may only be called on unicode type arrays')
        points = _native(2, '@' + str(len(self)) + 'I', self._buffer)
        return ''.join(map(chr, points))

    def fromfile(self, file, n):
        n = operator.index(n)
        if n < 0:
            raise ValueError('negative count')
        raw = file.read(n * self._itemsize)
        self.frombytes(raw)
        if len(raw) != n * self._itemsize:
            raise EOFError("read() didn't return enough bytes")

    def tofile(self, file):
        file.write(self.tobytes())

    def buffer_info(self):
        address, nbytes = _buffer_info(self._buffer)
        return (address, nbytes // self._itemsize)

    def __repr__(self):
        if not len(self):
            return 'array(' + repr(self._typecode) + ')'
        return 'array(' + repr(self._typecode) + ', ' + repr(self.tounicode() if self._typecode in 'uw' else self.tolist()) + ')'

    def __eq__(self, other):
        if not isinstance(other, array):
            return NotImplemented
        return self.tolist() == other.tolist()

    def __lt__(self, other):
        if not isinstance(other, array):
            return NotImplemented
        return self.tolist() < other.tolist()

    def __le__(self, other):
        return self == other or self < other

    def __gt__(self, other):
        if not isinstance(other, array):
            return NotImplemented
        return other < self

    def __ge__(self, other):
        return self == other or self > other

    def __add__(self, other):
        if not isinstance(other, array):
            raise TypeError('can only append array (not "' + type(other).__name__ + '") to array')
        if self._typecode != other.typecode:
            raise TypeError('bad argument type for built-in operation')
        return array(self._typecode, self.tobytes() + other.tobytes())

    def __iadd__(self, other):
        if not isinstance(other, array):
            raise TypeError('can only extend array with array')
        self.extend(other)
        return self

    def __mul__(self, count):
        count = operator.index(count)
        if len(self) and count > sys.maxsize // self._itemsize // len(self):
            raise MemoryError
        return array(self._typecode, self.tobytes() * count)

    def __rmul__(self, count):
        return self * count

    def __imul__(self, count):
        buffer = self._buffer
        count = operator.index(count)
        if len(self) and count > sys.maxsize // self._itemsize // len(self):
            raise MemoryError
        raw = self.tobytes() * count
        self._resize_check(len(raw))
        buffer[:] = raw
        return self

    def __copy__(self):
        return array(self._typecode, self.tobytes())

    def __deepcopy__(self, memo):
        return self.__copy__()

    def __reduce_ex__(self, protocol):
        state = {k: v for k, v in getattr(self, '__dict__', {}).items() if k not in ('_buffer', '_typecode', '_itemsize')}
        if protocol < 3:
            return (type(self), (self._typecode, self.tolist()), state or None)
        codes = {'B': 0, 'b': 1, 'H': 2, 'h': 4, 'I': 6, 'i': 8, 'L': 10, 'l': 12, 'Q': 10, 'q': 12, 'f': 14, 'd': 16, 'u': 20, 'w': 20}
        return (_array_reconstructor, (type(self), self._typecode, codes[self._typecode], self.tobytes()), state or None)

ArrayType = array

class _ArrayIterator:
    def __init__(self, value, index=0):
        self._array = value
        self._index = index
        self._done = False
    def __iter__(self):
        return self
    def __next__(self):
        if self._done or self._index >= len(self._array):
            self._done = True
            raise StopIteration
        value = self._array[self._index]
        self._index += 1
        return value
    def __length_hint__(self):
        return 0 if self._done else max(0, len(self._array) - self._index)
    def __reduce__(self):
        if self._done:
            return (iter, (array(self._array.typecode),))
        return (iter, (self._array,), self._index)
    def __setstate__(self, index):
        self._index = max(0, operator.index(index))


def _array_reconstructor(cls, typecode, mformat, items):
    if not isinstance(cls, type) or not issubclass(cls, array):
        raise TypeError('first argument must be a sub-type of array.array')
    if not isinstance(typecode, str) or len(typecode) != 1:
        raise TypeError('array() argument 1 must be a unicode character')
    if typecode not in typecodes:
        raise ValueError('bad typecode')
    mformat = operator.index(mformat)
    if not isinstance(items, bytes):
        raise TypeError('fourth argument should be bytes')
    formats = ['B', 'b', '<H', '>H', '<h', '>h', '<I', '>I', '<i', '>i', '<Q', '>Q', '<q', '>q', '<f', '>f', '<d', '>d']
    if mformat < 0 or mformat > 21:
        raise ValueError('third argument must be a valid machine format code')
    result = array.__new__(cls, typecode)
    native_codes = {'B': 0, 'b': 1, 'H': 2, 'h': 4, 'I': 6, 'i': 8, 'L': 10, 'l': 12, 'Q': 10, 'q': 12, 'f': 14, 'd': 16, 'u': 20, 'w': 20}
    if mformat == native_codes[typecode]:
        result.frombytes(items)
        return result
    if mformat >= 18:
        encoding = ['utf-16-le', 'utf-16-be', 'utf-32-le', 'utf-32-be'][mformat - 18]
        result.fromunicode(items.decode(encoding))
    else:
        fmt = formats[mformat]
        if len(items) % struct.calcsize(fmt):
            raise ValueError('string length not a multiple of item size')
        for value in struct.iter_unpack(fmt, items):
            result.append(value[0])
    return result

import collections.abc
collections.abc.MutableSequence.register(array)
