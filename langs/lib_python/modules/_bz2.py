# Lumen's own _bz2 extension module: the compressor and decompressor objects
# used by the release bz2 module, backed by the native bzip2 codec.
from operator import index as _index
from zlib import _buffer

_codec = __zlib_native__

class BZ2Compressor:
    def __init__(self, compresslevel=9, /):
        level = _index(compresslevel)
        if not 1 <= level <= 9:
            raise ValueError('compresslevel must be between 1 and 9')
        self._handle = _codec(32, 0, b'', level, 0, 0, 0, 0)[5]
        self._flushed = False

    def compress(self, data, /):
        data = _buffer(data)
        if self._flushed:
            raise ValueError('Compressor has been flushed')
        return _codec(33, self._handle, data, 0, 0, 0, 0, 0)[1]

    def flush(self):
        if self._flushed:
            raise ValueError('Repeated call to flush()')
        self._flushed = True
        packed = _codec(34, self._handle, b'', 0, 0, 0, 0, 0)[1]
        _codec(38, self._handle, b'', 0, 0, 0, 0, 0)
        return packed

    def __reduce__(self):
        raise TypeError("cannot pickle '_bz2.BZ2Compressor' object")

class BZ2Decompressor:
    def __new__(cls):
        self = object.__new__(cls)
        self._handle = _codec(35, 0, b'', 0, 0, 0, 0, 0)[5]
        self._eof = False
        self._failed = False
        self._unused = b''
        self._needs_input = True
        return self

    @property
    def eof(self):
        return self._eof

    @property
    def unused_data(self):
        return self._unused

    @property
    def needs_input(self):
        return self._needs_input

    def decompress(self, data, max_length=-1):
        data = _buffer(data)
        limit = _index(max_length)
        if self._eof:
            raise EOFError('End of stream already reached')
        if self._failed:
            raise ValueError('Decompressor is in an error state due to a previous error')
        status, output, _, ended, text, stalled = _codec(36, self._handle, data, limit if limit >= 0 else -1, 0, 0, 0, 0)
        if status:
            self._failed = True
            self._needs_input = False
            raise OSError(text)
        if ended:
            self._eof = True
            self._unused = _codec(37, self._handle, b'', 0, 0, 0, 0, 0)[1]
            _codec(38, self._handle, b'', 0, 0, 0, 0, 0)
        self._needs_input = bool(stalled) and not ended
        return output

    def __reduce__(self):
        raise TypeError("cannot pickle '_bz2.BZ2Decompressor' object")
