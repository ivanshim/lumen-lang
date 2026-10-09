"""Native codec helpers used by the Python encoding registry."""
def _normalize_encoding(encoding):
    if not isinstance(encoding, str):
        raise TypeError('encoding must be a string')
    result = []
    separator = False
    for character in encoding:
        if character.isalnum() or character == '.':
            if separator and result:
                result.append('_')
            if character.isascii():
                result.append(character)
            separator = False
        else:
            separator = True
    return ''.join(result)

import _codec_runtime as _runtime

register = _runtime.register
unregister = _runtime.unregister
lookup = _runtime.lookup
register_error = _runtime.register_error
lookup_error = _runtime.lookup_error
encode = _runtime.encode
decode = _runtime.decode

# Preserve C-function attribute behavior when codecs attach helpers to classes.
class _CodecCall:
    # Keep the underlying adapter callable without a descriptor binding.
    def __init__(self, function, name=None):
        self.function = function
        self.__name__ = name or function.__name__
    # Pass codec arguments through unchanged.
    def __call__(self, *args, **kwargs):
        return self.function(*args, **kwargs)
    # Restore native codec entry points by their public module identity.
    def __reduce__(self):
        return (_get_codec_call, (self.__name__,))

# Retrieve the registered global callable rather than duplicating native state.
def _get_codec_call(name):
    return globals()[name]

# Require a text object for a Unicode encoder.
def _text(input):
    if not isinstance(input, str):
        raise TypeError('argument must be str, not ' + type(input).__name__)
    return input

# Accept the buffer protocol while refusing bytes constructor sizes.
def _buffer(input):
    try:
        view = memoryview(input)
    except TypeError:
        if hasattr(type(input), '__buffer__'):
            raise
        raise TypeError("a bytes-like object is required, not '%s'" % type(input).__name__) from None
    return view.tobytes()

# C codec entry points accept None as the default error policy.
def _error_argument(errors, function):
    if errors is None:
        return 'strict'
    if not isinstance(errors, str):
        raise TypeError(function + '() argument 2 must be str or None, not ' + type(errors).__name__)
    if '\x00' in errors:
        raise ValueError('embedded null character')
    return errors

# Return a stateless encoder with the C codec tuple interface.
def _encoder(name):
    # Encode a Unicode input and report characters consumed.
    def encode(input, errors='strict'):
        input = _text(input)
        errors = _error_argument(errors, name + '_encode')
        return _runtime._encode(input, name, errors), len(input)
    return _CodecCall(encode, name + '_encode')

# Find the prefix that ends before an unfinished escaped character.
def _escape_end(data, raw):
    i = 0
    while i < len(data):
        start = i
        if data[i] != 92:
            i += 1
            continue
        i += 1
        if i == len(data):
            return start
        c = data[i]
        i += 1
        if c in (117, 85) or c == 120 and not raw:
            width = 4 if c == 117 else 8 if c == 85 else 2
            digits = 0
            while i < len(data) and digits < width and chr(data[i]) in '0123456789abcdefABCDEF':
                i += 1
                digits += 1
            if digits < width and i == len(data):
                return start
        elif not raw and c == 78:
            if i == len(data):
                return start
            if data[i] == 123:
                end = data.find(b'}', i)
                if end == -1:
                    return start
                i = end + 1
        elif not raw and 48 <= c <= 55:
            digits = 1
            while i < len(data) and digits < 3 and 48 <= data[i] <= 55:
                i += 1
                digits += 1
    return len(data)

# Return a decoder with byte consumption and non-final buffering.
def _decoder(name, default_final=False):
    # Decode only complete characters when the caller has more bytes to supply.
    def decode(input, errors='strict', final=default_final):
        data = input.encode('utf-8') if isinstance(input, str) and name in ('unicode_escape', 'raw_unicode_escape') else _buffer(input)
        errors = _error_argument(errors, name + '_decode')
        if name.startswith('utf_16') or name.startswith('utf_32'):
            return _runtime._wide_decode(data, name, errors, final)
        if name == 'utf_8':
            decoder = _runtime._IncrementalDecoder(name, errors)
            output = decoder.decode(data, final)
            return output, len(data) - len(decoder.buffer)
        if name == 'utf_7':
            return _runtime._utf7_decode(data, errors, final)
        end = len(data)
        if not final and name in ('unicode_escape', 'raw_unicode_escape'):
            end = _escape_end(data, name == 'raw_unicode_escape')
        return _runtime._decode(data[:end], name, errors), end
    return _CodecCall(decode, name + '_decode')

ascii_encode = _encoder('ascii')
ascii_decode = _decoder('ascii')
latin_1_encode = _encoder('latin_1')
latin_1_decode = _decoder('latin_1')
utf_8_encode = _encoder('utf_8')
utf_8_decode = _decoder('utf_8')
utf_7_encode = _encoder('utf_7')
utf_7_decode = _decoder('utf_7')
utf_16_encode = _encoder('utf_16')
utf_16_le_encode = _encoder('utf_16_le')
utf_16_be_encode = _encoder('utf_16_be')
utf_16_le_decode = _decoder('utf_16_le')
utf_16_be_decode = _decoder('utf_16_be')
utf_32_encode = _encoder('utf_32')
utf_32_le_encode = _encoder('utf_32_le')
utf_32_be_encode = _encoder('utf_32_be')
utf_32_le_decode = _decoder('utf_32_le')
utf_32_be_decode = _decoder('utf_32_be')
unicode_escape_encode = _encoder('unicode_escape')
unicode_escape_decode = _decoder('unicode_escape', True)
raw_unicode_escape_encode = _encoder('raw_unicode_escape')
raw_unicode_escape_decode = _decoder('raw_unicode_escape', True)
charmap_build = _CodecCall(_runtime.charmap_build)
charmap_encode = _CodecCall(_runtime.charmap_encode)
charmap_decode = _CodecCall(_runtime.charmap_decode)

# Detect a BOM only when byte order was not supplied by the caller.
def _wide_ex(input, errors, byteorder, final, width):
    data = _buffer(input)
    errors = _error_argument(errors, 'utf_16_ex_decode' if width == 2 else 'utf_32_ex_decode')
    skip = 0
    prefix = 'utf_16' if width == 2 else 'utf_32'
    if byteorder == 0:
        le = _runtime.BOM_UTF16_LE if width == 2 else _runtime.BOM_UTF32_LE
        be = _runtime.BOM_UTF16_BE if width == 2 else _runtime.BOM_UTF32_BE
        if data.startswith(le):
            byteorder = -1
            skip = width
        elif data.startswith(be):
            byteorder = 1
            skip = width
    little = byteorder < 0 or byteorder == 0 and _runtime.sys.byteorder == 'little'
    name = prefix + ('_le' if little else '_be')
    output, consumed = _runtime._wide_decode(data, name, errors, final, skip)
    return output, consumed, byteorder

# Decode UTF-16 and return the detected stream byte order.
def utf_16_ex_decode(input, errors='strict', byteorder=0, final=False):
    return _wide_ex(input, errors, byteorder, final, 2)

# Decode UTF-32 and return the detected stream byte order.
def utf_32_ex_decode(input, errors='strict', byteorder=0, final=False):
    return _wide_ex(input, errors, byteorder, final, 4)

# Decode UTF-16 with native byte order unless a BOM selects another order.
def utf_16_decode(input, errors='strict', final=False):
    errors = _error_argument(errors, 'utf_16_decode')
    output, consumed, order = utf_16_ex_decode(input, errors, 0, final)
    return output, consumed

# Decode UTF-32 with native byte order unless a BOM selects another order.
def utf_32_decode(input, errors='strict', final=False):
    errors = _error_argument(errors, 'utf_32_decode')
    output, consumed, order = utf_32_ex_decode(input, errors, 0, final)
    return output, consumed

# Convert a buffer or UTF-8 text into the read-buffer codec's byte result.
def readbuffer_encode(input, errors='strict'):
    errors = _error_argument(errors, 'readbuffer_encode')
    data = input.encode('utf-8') if isinstance(input, str) else _buffer(input)
    return data, len(data)

# Escape bytes using Python's byte literal escape alphabet.
def escape_encode(input, errors='strict'):
    errors = _error_argument(errors, 'escape_encode')
    if not isinstance(input, bytes):
        raise TypeError('escape_encode() argument 1 must be bytes, not ' + type(input).__name__)
    data = _buffer(input)
    output = []
    escapes = {9: b'\\t', 10: b'\\n', 13: b'\\r', 39: b"\\'", 92: b'\\\\'}
    for n in data:
        if n in escapes:
            output.append(escapes[n])
        elif 32 <= n < 127:
            output.append(bytes([n]))
        else:
            output.append(('\\x%02x' % n).encode('ascii'))
    return b''.join(output), len(data)

# Interpret byte escape sequences, including the selected malformed-hex policy.
def escape_decode(input, errors='strict'):
    errors = _error_argument(errors, 'escape_decode')
    data = input.encode('utf-8') if isinstance(input, str) else _buffer(input)
    output = []
    warning = None
    i = 0
    while i < len(data):
        n = data[i]
        i += 1
        if n != 92:
            output.append(n)
            continue
        start = i - 1
        if i == len(data):
            raise ValueError('Trailing \\ in string')
        c = chr(data[i])
        i += 1
        if c == 'x':
            end = i
            while end < len(data) and end < i + 2 and chr(data[end]) in '0123456789abcdefABCDEF':
                end += 1
            if end == i + 2:
                output.append(int(data[i:end], 16))
            elif errors == 'replace':
                output.append(63)
            elif errors != 'ignore':
                if errors != 'strict':
                    raise ValueError('decoding error; unknown error handling code: ' + errors)
                raise ValueError('invalid \\x escape at position %d' % start)
            i = end
        elif c in _runtime._escape_decode:
            output.append(ord(_runtime._escape_decode[c]))
        elif c == '\n':
            pass
        elif c in '01234567':
            end = i
            while end < len(data) and end - i < 2 and chr(data[end]) in '01234567':
                end += 1
            octal = int(data[i - 1:end], 8)
            if octal > 255 and warning is None:
                warning = 'b"\\%s" is an invalid octal escape sequence. Such sequences will not work in the future.' % data[i - 1:end].decode('ascii')
            output.append(octal & 255)
            i = end
        else:
            if warning is None:
                warning = 'b"\\%s" is an invalid escape sequence. Such sequences will not work in the future.' % c
            output.extend([92, ord(c)])
    if warning is not None:
        import warnings
        warnings.warn(warning, DeprecationWarning, stacklevel=2)
    return bytes(output), len(data)
