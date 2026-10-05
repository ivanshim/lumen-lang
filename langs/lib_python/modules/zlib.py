# Python facade for the native codec; semantics from CPython v3.14.8
# Modules/zlibmodule.c (PSF License, tests/python/LICENSE).
_codec = __zlib_native__
from operator import index as _index

DEFLATED = 8
MAX_WBITS = 15
DEF_MEM_LEVEL = 8
DEF_BUF_SIZE = 16384
Z_NO_FLUSH = 0
Z_PARTIAL_FLUSH = 1
Z_SYNC_FLUSH = 2
Z_FULL_FLUSH = 3
Z_FINISH = 4
Z_BLOCK = 5
Z_TREES = 6
Z_DEFAULT_COMPRESSION = -1
Z_NO_COMPRESSION = 0
Z_BEST_SPEED = 1
Z_BEST_COMPRESSION = 9
Z_DEFAULT_STRATEGY = 0
Z_FILTERED = 1
Z_HUFFMAN_ONLY = 2
Z_RLE = 3
Z_FIXED = 4
ZLIB_VERSION = _codec(0, 0, b'', 0, 0, 0, 0, 0)[4]
ZLIB_RUNTIME_VERSION = ZLIB_VERSION

_UNSET = object()

class error(Exception):
    pass

def _buffer(data):
    if isinstance(data, bytes):
        return data
    if isinstance(data, bytearray):
        return bytes(data)
    from array import array
    if not isinstance(data, (memoryview, array)) and not hasattr(type(data), '__buffer__'):
        raise TypeError("a bytes-like object is required, not '" + type(data).__name__ + "'")
    from builtins import _buffer_view
    view = _buffer_view(data, 0)
    try:
        if not view.c_contiguous:
            raise BufferError('memoryview: underlying buffer is not C-contiguous')
        return view.tobytes()
    finally:
        view.release()

def _indexed(value):
    if not isinstance(value, int) and not hasattr(type(value), '__index__'):
        raise TypeError("'" + type(value).__name__ + "' object cannot be interpreted as an integer")
    return _index(value)

def _dictionary_buffer_check(value):
    # PyObject_CheckBuffer checks the protocol, without acquiring a view.
    # In particular, released and strided memoryviews still support it.
    from array import array
    if not isinstance(value, (bytes, bytearray, memoryview, array)) and not hasattr(type(value), '__buffer__'):
        raise TypeError('zdict argument must support the buffer protocol')

def _integer(value):
    n = _indexed(value)
    if n < -2147483648 or n > 2147483647:
        raise OverflowError('Python int too large to convert to C int')
    return n

def _size(value):
    n = _indexed(value)
    if n < -9223372036854775808 or n > 9223372036854775807:
        raise OverflowError('Python int too large to convert to C ssize_t')
    return n

def _invoke(op, handle=0, data=b'', a=0, b=0, c=0, d=0, e=0):
    return _codec(op, handle, data, a, b, c, d, e)

def _raise(reply, context):
    code = reply[0]
    if code:
        msg = 'Error %d %s' % (code, context)
        if reply[4]:
            msg += ': ' + reply[4]
        raise error(msg)

def crc32(data, value=0, /):
    return _invoke(7, data=_buffer(data), a=_indexed(value) & 0xffffffff)[5]

def adler32(data, value=1, /):
    return _invoke(8, data=_buffer(data), a=_indexed(value) & 0xffffffff)[5]

class Compress:
    def __init__(self, level=-1, method=DEFLATED, wbits=MAX_WBITS,
                 memLevel=DEF_MEM_LEVEL, strategy=Z_DEFAULT_STRATEGY, zdict=_UNSET):
        level = _integer(level)
        method = _integer(method)
        window = _integer(wbits)
        memory = _integer(memLevel)
        strategy = _integer(strategy)
        dictionary = b'' if zdict is _UNSET else _buffer(zdict)
        if len(dictionary) > 4294967295:
            raise OverflowError('zdict length does not fit in an unsigned int')
        reply = _invoke(1, int(zdict is not _UNSET), data=dictionary,
                        a=level, b=method, c=window, d=memory, e=strategy)
        if reply[5] == -1:
            raise ValueError('Invalid dictionary' if reply[0] == -2 else 'deflateSetDictionary()')
        if reply[0] == -2:
            raise ValueError('Invalid initialization option')
        _raise(reply, 'while creating compression object')
        self._handle = reply[5]
        self._finished = False

    def compress(self, data, /):
        data = _buffer(data)
        reply = _invoke(3, self._handle, data)
        _raise(reply, 'while compressing data')
        return reply[1]

    def flush(self, mode=Z_FINISH, /):
        mode = _integer(mode)
        if mode == Z_NO_FLUSH:
            return b''
        reply = _invoke(3, self._handle, a=mode)
        _raise(reply, 'while flushing')
        if reply[3]:
            self._finished = True
            _invoke(6, self._handle)
        return reply[1]

    def copy(self):
        if self._finished:
            raise ValueError('Inconsistent stream state')
        reply = _invoke(5, self._handle)
        _raise(reply, 'while copying compression object')
        new = object.__new__(type(self))
        new._handle = reply[5]
        new._finished = False
        return new

    def __copy__(self):
        return self.copy()

    def __deepcopy__(self, memo, /):
        return self.copy()

    def __reduce_ex__(self, protocol):
        raise TypeError("cannot pickle 'zlib.Compress' object")

    def __del__(self):
        if hasattr(self, '_handle'):
            _invoke(6, self._handle)

class Decompress:
    def __init__(self, wbits=MAX_WBITS, zdict=_UNSET):
        window = _integer(wbits)
        if zdict is not _UNSET:
            _dictionary_buffer_check(zdict)
        reply = _invoke(2, a=window)
        if reply[0] == -2:
            raise ValueError('Invalid initialization option')
        _raise(reply, 'while creating decompression object')
        if zdict is not _UNSET and window < 0:
            try:
                dictionary = _buffer(zdict)
                if len(dictionary) > 4294967295:
                    raise OverflowError('zdict length does not fit in an unsigned int')
                _raise(_invoke(9, reply[5], dictionary), 'while setting zdict')
            except:
                _invoke(6, reply[5])
                raise
        self._dictionary = zdict
        self._handle = reply[5]
        self._eof = False
        self._unused_data = b''
        self._unconsumed_tail = b''
        self._finished = False

    @property
    def eof(self):
        return self._eof

    @property
    def unused_data(self):
        return self._unused_data

    @property
    def unconsumed_tail(self):
        return self._unconsumed_tail

    def _decode(self, data, flush, maximum):
        reply = _invoke(4, self._handle, data, a=flush, b=maximum)
        # v3.14.8's private decompressor returns NULL without an exception
        # on Z_NEED_DICT. Preserve that SystemError instead of accepting data
        # the pinned implementation refuses.
        if reply[0] == 2 and getattr(self, '_dictionary_null_error', False):
            raise SystemError('error return without exception set')
        if reply[0] == 2 and self._dictionary is not _UNSET:
            dictionary = _buffer(self._dictionary)
            if len(dictionary) > 4294967295:
                raise OverflowError('zdict length does not fit in an unsigned int')
            setting = _invoke(9, self._handle, dictionary)
            _raise(setting, 'while setting zdict')
            rest = _invoke(4, self._handle, data[reply[2]:], a=flush, b=maximum)
            reply = (rest[0], reply[1] + rest[1], reply[2] + rest[2], rest[3], rest[4], rest[5])
        return reply

    def decompress(self, data, /, max_length=0):
        data = _buffer(data)
        maximum = _size(max_length)
        if maximum < 0:
            raise ValueError('max_length must be non-negative')
        if self._finished:
            _raise((-2, b'', 0, False, 'inconsistent stream state', 0), 'while decompressing data')
        if self._eof:
            self._unused_data += data
            self._unconsumed_tail = b''
            return b''
        reply = self._decode(data, Z_NO_FLUSH, maximum)
        tail = data[reply[2]:]
        if reply[3]:
            self._eof = True
            self._unused_data += tail
            self._unconsumed_tail = b''
        else:
            self._unconsumed_tail = tail
        _raise(reply, 'while decompressing data')
        return reply[1]

    def flush(self, length=DEF_BUF_SIZE, /):
        if _size(length) <= 0:
            raise ValueError('length must be greater than zero')
        if self._finished:
            return b''
        reply = self._decode(self._unconsumed_tail, Z_FINISH, 0)
        tail = self._unconsumed_tail[reply[2]:]
        if reply[3]:
            self._eof = True
            self._unused_data += tail
            self._unconsumed_tail = b''
            self._finished = True
            _invoke(6, self._handle)
        else:
            self._unconsumed_tail = tail
        _raise(reply, 'while decompressing data')
        return reply[1]

    def copy(self):
        if self._finished:
            raise ValueError('Inconsistent stream state')
        reply = _invoke(5, self._handle)
        _raise(reply, 'while copying decompression object')
        new = object.__new__(type(self))
        new._handle = reply[5]
        new._dictionary = self._dictionary
        new._eof = self._eof
        new._unused_data = self._unused_data
        new._unconsumed_tail = self._unconsumed_tail
        new._finished = False
        return new

    def __copy__(self):
        return self.copy()

    def __deepcopy__(self, memo, /):
        return self.copy()

    def __reduce_ex__(self, protocol):
        raise TypeError("cannot pickle 'zlib.Decompress' object")

    def __del__(self):
        if hasattr(self, '_handle'):
            _invoke(6, self._handle)

def compressobj(level=-1, method=DEFLATED, wbits=MAX_WBITS,
                memLevel=DEF_MEM_LEVEL, strategy=Z_DEFAULT_STRATEGY, zdict=_UNSET):
    return Compress(level, method, wbits, memLevel, strategy, zdict)

def decompressobj(wbits=MAX_WBITS, zdict=_UNSET):
    return Decompress(wbits, zdict)

def compress(data, /, level=-1, wbits=MAX_WBITS):
    data = _buffer(data)
    level = _integer(level)
    wbits = _integer(wbits)
    try:
        obj = compressobj(level, wbits=wbits)
    except ValueError:
        raise error('Bad compression level')
    return obj.compress(data) + obj.flush()

def decompress(data, /, wbits=MAX_WBITS, bufsize=DEF_BUF_SIZE):
    data = _buffer(data)
    wbits = _integer(wbits)
    if _size(bufsize) < 0:
        raise ValueError('bufsize must be non-negative')
    try:
        obj = decompressobj(wbits)
    except ValueError:
        raise error('Error -2 while preparing to decompress data: inconsistent stream state')
    output = obj.decompress(data)
    if not obj.eof:
        raise error('Error -5 while decompressing data: incomplete or truncated stream')
    return output

class _ZlibDecompressor:
    def __init__(self, wbits=MAX_WBITS, zdict=_UNSET):
        window = _integer(wbits)
        self._decoder = decompressobj(window, zdict) if window < 0 else decompressobj(window)
        self._zdict = zdict
        self._decoder._dictionary_null_error = True
        self._pending = b''
        self._needs_input = True

    @property
    def eof(self):
        return self._decoder.eof

    @property
    def unused_data(self):
        return self._decoder.unused_data

    @property
    def needs_input(self):
        return self._needs_input

    def decompress(self, data, max_length=-1):
        if self.eof:
            raise EOFError('End of stream already reached')
        data = self._pending + _buffer(data)
        maximum = _size(max_length)
        if maximum == 0:
            self._pending = data
            self._needs_input = not data
            return b''
        result = self._decoder.decompress(data, max(0, maximum))
        self._pending = self._decoder.unconsumed_tail
        self._needs_input = not self._pending and not self.eof
        return result

    def __reduce_ex__(self, protocol):
        raise TypeError("cannot pickle 'zlib._ZlibDecompressor' object")
