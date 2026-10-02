# Python interface to the native CPython v3.14.8 binary format operations.
import operator
import sys

__doc__ = 'Functions to convert between Python values and C structs.'
__all__ = ['calcsize', 'pack', 'pack_into', 'unpack', 'unpack_from', 'iter_unpack', 'Struct', 'error']
_native = __struct_native

class error(Exception):
    pass

_cache = {}
def _clearcache():
    _cache.clear()
    _native(4, '')

def _format(value):
    if isinstance(value, bytes):
        return value.decode('ascii')
    if not isinstance(value, str):
        raise TypeError('Struct() argument 1 must be a str or bytes object')
    value.encode('ascii')
    return value

def _read_view(buffer):
    view = memoryview(buffer)
    try:
        return view.cast('B')
    except TypeError:
        raise BufferError('memoryview: underlying buffer is not C-contiguous')

def _call(action, format, values=None):
    try:
        return _native(action, format, values) if values is not None else _native(action, format)
    except ValueError as exc:
        raise error(exc.args[0])

def _get(format):
    if isinstance(format, str) and format in _cache:
        return _cache[format]
    format = _format(format)
    if format not in _cache:
        if len(_cache) >= 100:
            _clearcache()
        _cache[format] = Struct(format)
    return _cache[format]

def _argument_codes(fields):
    for code, repeat in fields:
        if code != 'x':
            for index in range(1 if code in 'sp' else repeat):
                yield code

class Struct:
    def __new__(cls, *args, **kwargs):
        self = object.__new__(cls)
        self._format = None
        self._size = -1
        self._fields = ()
        self._codes = ()
        self._count = 0
        return self

    def _ready(self):
        if self._format is None:
            raise RuntimeError('Struct object is uninitialized')

    def __sizeof__(self):
        self._ready()
        raise NotImplementedError('Struct storage size is not exposed')

    def __init__(self, format):
        format = _format(format)
        size, fields = _call(0, format)
        self._format = format
        self._size = size
        self._fields = fields
        self._count = sum(0 if code == 'x' else 1 if code in 'sp' else repeat for code, repeat in fields)
        self._codes = tuple(_argument_codes(fields)) if self._count <= 256 else None

    @property
    def format(self):
        self._ready()
        return self._format

    @property
    def size(self):
        return self._size

    def __repr__(self):
        return 'Struct(' + repr(self.format) + ')'

    def _arguments(self, values):
        expected = self._count
        if len(values) != expected:
            raise error('pack expected ' + str(expected) + ' items for packing (got ' + str(len(values)) + ')')
        normalized = []
        codes = self._codes if self._codes is not None else _argument_codes(self._fields)
        for code, value in zip(codes, values):
            if code in 'bBhHiIlLqQnNP':
                if not isinstance(value, int) and not hasattr(type(value), '__index__'):
                    raise error('required argument is not an integer')
                if not isinstance(value, int):
                    value = operator.index(value)
            elif code in 'efd':
                if not isinstance(value, (int, float)) and not hasattr(type(value), '__float__') and not hasattr(type(value), '__index__'):
                    raise error('required argument is not a float')
                try:
                    if not isinstance(value, float):
                        value = float(value)
                except (TypeError, ValueError):
                    raise error('required argument is not a float')
            elif code in 'FD':
                if not isinstance(value, (int, float, complex)) and not hasattr(type(value), '__complex__') and not hasattr(type(value), '__float__') and not hasattr(type(value), '__index__'):
                    raise error('required argument is not a complex')
                try:
                    value = complex(value)
                except TypeError:
                    raise error('required argument is not a complex')
                value = (value.real, value.imag)
            elif code == '?':
                value = bool(value)
            normalized.append(value)
        return tuple(normalized)

    def pack(self, *values):
        self._ready()
        return _call(1, self._format, self._arguments(values))

    def unpack(self, buffer):
        self._ready()
        format = self._format
        raw = buffer if isinstance(buffer, (bytes, bytearray)) else _read_view(buffer).tobytes()
        result = _call(2, format, raw)
        if 'F' not in format and 'D' not in format:
            return result
        converted = []
        i = 0
        for code, count in self._fields:
            if code == 'x':
                continue
            for j in range(1 if code in 'sp' else count):
                value = result[i]
                i += 1
                converted.append(complex(*value) if code in 'FD' else value)
        return tuple(converted)

    def _offset(self, length, offset, packing):
        offset = operator.index(offset)
        if offset > sys.maxsize or offset < -sys.maxsize - 1:
            if packing:
                raise IndexError("cannot fit 'int' into an index-sized integer")
            raise OverflowError('Python int too large to convert to C ssize_t')
        original = offset
        if offset < 0:
            if offset + self.size > 0:
                raise error(('no space to pack ' if packing else 'not enough data to unpack ') + str(self.size) + ' bytes at offset ' + str(offset))
            offset += length
            if offset < 0:
                raise error('offset ' + str(original) + ' out of range for ' + str(length) + '-byte buffer')
        if offset > length - self.size:
            action = 'packing' if packing else 'unpacking'
            function = 'pack_into' if packing else 'unpack_from'
            raise error(function + ' requires a buffer of at least ' + str(offset + self.size) + ' bytes for ' + action + ' ' + str(self.size) + ' bytes at offset ' + str(original) + ' (actual buffer size is ' + str(length) + ')')
        return offset

    def pack_into(self, buffer, offset, *values):
        self._ready()
        view = memoryview(buffer)
        if view.readonly:
            raise TypeError('argument must be read-write bytes-like object, not ' + type(buffer).__name__)
        try:
            byteview = view.cast('B')
        except TypeError:
            raise TypeError('argument must be read-write bytes-like object, not memoryview')
        at = self._offset(view.nbytes, offset, True)
        raw = self.pack(*values)
        byteview[at:at + self.size] = raw

    def unpack_from(self, buffer, offset=0):
        self._ready()
        view = _read_view(buffer)
        at = self._offset(view.nbytes, offset, False)
        return self.unpack(view.cast('B')[at:at + self.size])

    def iter_unpack(self, buffer):
        self._ready()
        view = _read_view(buffer)
        if self.size == 0:
            raise error('cannot iteratively unpack with a struct of length 0')
        if view.nbytes % self.size:
            raise error('iterative unpacking requires a buffer of a multiple of ' + str(self.size) + ' bytes')
        return _UnpackIterator(self, view.cast('B'))

class _UnpackIterator:
    def __init__(self, struct, view):
        self._struct = struct
        self._view = view
        self._at = 0
    def __iter__(self):
        return self
    def __next__(self):
        if self._at >= self._view.nbytes:
            raise StopIteration
        value = self._struct.unpack_from(self._view, self._at)
        self._at += self._struct.size
        return value
    def __length_hint__(self):
        return (self._view.nbytes - self._at) // self._struct.size

def calcsize(format):
    return _get(format).size

def pack(format, *values):
    return _get(format).pack(*values)

def unpack(format, buffer):
    return _get(format).unpack(buffer)

def pack_into(format, buffer, offset, *values):
    return _get(format).pack_into(buffer, offset, *values)

def unpack_from(format, buffer, offset=0):
    return _get(format).unpack_from(buffer, offset)

def iter_unpack(format, buffer):
    return _get(format).iter_unpack(buffer)
