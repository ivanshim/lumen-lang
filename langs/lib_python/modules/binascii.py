# Hexadecimal conversions follow CPython v3.14.8 Modules/binascii.c (PSF License).
class Error(ValueError):
    pass

class Incomplete(Exception):
    pass

def unhexlify(data, /):
    if isinstance(data, str):
        try:
            data.encode('ascii')
        except UnicodeEncodeError:
            raise ValueError('string argument should contain only ASCII characters')
    else:
        data = memoryview(data).tobytes().decode('latin-1')
    if len(data) % 2:
        raise Error('Odd-length string')
    for code in data:
        if code not in '0123456789abcdefABCDEF':
            raise Error('Non-hexadecimal digit found')
    return bytes.fromhex(data)

def hexlify(data, sep=None, bytes_per_sep=1):
    data = memoryview(data).tobytes()
    if isinstance(sep, bytes):
        sep = sep.decode('ascii')
    return (data.hex() if sep is None else data.hex(sep, bytes_per_sep)).encode('ascii')

a2b_hex = unhexlify
b2a_hex = hexlify
