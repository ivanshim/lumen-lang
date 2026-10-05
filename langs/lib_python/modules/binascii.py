# Python entry points for CPython v3.14.8 Modules/binascii.c; PSF License.
# The conversions and checksums are supplied by each full kernel.
import operator

class Error(ValueError):
    pass

class Incomplete(Exception):
    pass

Error.__module__ = 'binascii'
Incomplete.__module__ = 'binascii'

def _buffer(data, ascii=False):
    if ascii and isinstance(data, str):
        if not data.isascii():
            raise ValueError('string argument should contain only ASCII characters')
        return data.encode('ascii')
    if isinstance(data, (bytes, bytearray)):
        return data
    view = memoryview(data)
    if not view.c_contiguous:
        raise BufferError('memoryview: underlying buffer is not C-contiguous')
    return view.tobytes()

def _convert(operation, data, *options):
    result = __binascii_native__(operation, data, *options)
    if isinstance(result, str):
        raise Error(result)
    return result

def a2b_base64(data, /, *, strict_mode=False):
    return _convert('decode64', _buffer(data, True), bool(strict_mode))

def b2a_base64(data, /, *, newline=True):
    return _convert('encode64', _buffer(data), bool(newline))

def a2b_uu(data, /):
    return _convert('decodeuu', _buffer(data, True))

def b2a_uu(data, /, *, backtick=False):
    return _convert('encodeuu', _buffer(data), bool(backtick))

def a2b_qp(data, header=False):
    return _convert('decodeqp', _buffer(data, True), bool(header))

def b2a_qp(data, quotetabs=False, istext=True, header=False):
    return _convert('encodeqp', _buffer(data), bool(quotetabs), bool(istext), bool(header))

_missing = object()

def hexlify(data, sep=_missing, bytes_per_sep=1):
    data = _buffer(data)
    group = operator.index(bytes_per_sep)
    if group < -2147483648 or group > 2147483647:
        raise OverflowError('Python int too large to convert to C int')
    if sep is not _missing:
        if len(sep) != 1:
            raise ValueError('sep must be length 1.')
        if not isinstance(sep, (str, bytes)):
            raise TypeError('sep must be str or bytes.')
        if isinstance(sep, str):
            if ord(sep) > 255:
                raise ValueError('sep must be ASCII.')
            sep = bytes([ord(sep)])
    return _convert('hex', data, None if sep is _missing else sep, group)

def b2a_hex(data, sep=_missing, bytes_per_sep=1):
    return hexlify(data, sep, bytes_per_sep)

def unhexlify(data, /):
    return _convert('unhex', _buffer(data, True))

def a2b_hex(data, /):
    return unhexlify(data)

def crc32(data, crc=0, /):
    return _convert('crc32', _buffer(data), operator.index(crc) & 4294967295)

def crc_hqx(data, crc, /):
    return _convert('hqx', _buffer(data), operator.index(crc) & 65535)
