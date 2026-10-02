# Buffer conversions corresponding to CPython v3.14.8 Modules/binascii.c.
_native_encoding = __crypto

class Error(ValueError):
    pass

class Incomplete(Exception):
    pass


def unhexlify(data, /):
    if isinstance(data, str):
        if not data.isascii():
            raise ValueError('string argument should contain only ASCII characters')
        data = data.encode('ascii')
    else:
        data = memoryview(data).tobytes()
    if len(data) % 2:
        raise Error('Odd-length string')
    values = []
    alphabet = b'0123456789abcdef'
    data = data.lower()
    for i in range(0, len(data), 2):
        if data[i] not in alphabet or data[i + 1] not in alphabet:
            raise Error('Non-hexadecimal digit found')
        values.append(alphabet.index(data[i]) * 16 + alphabet.index(data[i + 1]))
    return bytes(values)


def hexlify(data, /, sep=None, bytes_per_sep=1):
    data = memoryview(data).tobytes()
    if sep is None:
        return data.hex().encode('ascii')
    return data.hex(sep, bytes_per_sep).encode('ascii')


def b2a_base64(data, /, *, newline=True):
    answer = _native_encoding(3, memoryview(data).tobytes())
    if newline:
        answer += b'\n'
    return answer


b2a_hex = hexlify
a2b_hex = unhexlify
