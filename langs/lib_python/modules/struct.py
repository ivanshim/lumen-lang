# Packing cannot substitute text or a list for a byte value.
class error(Exception):
    pass

def pack(format, *values):
    raise 'NotImplementedError: struct.pack needs byte values'

def unpack(format, buffer):
    raise 'NotImplementedError: struct.unpack needs byte values'

def calcsize(format):
    if not isinstance(format, str):
        raise TypeError('Struct() argument 1 must be a str or bytes object')
    native = True
    if format[:1] in ('@', '=', '<', '>', '!'):
        native = format[0] == '@'
        format = format[1:]
    sizes = {'x': 1, 'c': 1, 'b': 1, 'B': 1, '?': 1,
             'h': 2, 'H': 2, 'i': 4, 'I': 4, 'l': 8 if native else 4,
             'L': 8 if native else 4, 'q': 8, 'Q': 8,
             'e': 2, 'f': 4, 'd': 8, 's': 1, 'p': 1}
    if native:
        sizes.update({'n': 8, 'N': 8, 'P': 8})
    size = 0
    count = ''
    for code in format:
        if code in '0123456789':
            count += code
            continue
        if code in ' \t\n\r\v\f' and not count:
            continue
        if code not in sizes:
            raise error('bad char in struct format')
        width = sizes[code]
        if native:
            size += (width - size % width) % width
        size += width * (int(count) if count else 1)
        count = ''
        if size > 9223372036854775807:
            raise error('total struct size too long')
    if count:
        raise error('repeat count given without format specifier')
    return size

class Struct:
    def __init__(self, format):
        raise 'NotImplementedError: Struct needs byte values'

def __getattr__(name):
    raise 'NotImplementedError: struct.' + name + ' needs byte values'
