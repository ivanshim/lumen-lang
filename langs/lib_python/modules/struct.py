# Packing cannot substitute text or a list for a byte value.
class error(Exception):
    pass

def pack(format, *values):
    if format == '<q' or format == '>q' or format == '!q':
        if len(values) != 1:
            raise error('pack expected 1 items for packing (got ' + str(len(values)) + ')')
        return int(values[0]).to_bytes(8, 'little' if format == '<q' else 'big', signed=True)
    if format == '<d' or format == '>d' or format == '!d':
        if len(values) != 1:
            raise error('pack expected 1 items for packing (got ' + str(len(values)) + ')')
        import math
        value = float(values[0])
        if math.isnan(value):
            bits = 9221120237041090560
        elif math.isinf(value):
            bits = 9218868437227405312
        else:
            mantissa, exponent = math.frexp(abs(value))
            if value == 0:
                bits = 0
            elif exponent < -1021:
                fraction = int(math.ldexp(mantissa, exponent + 1074))
                bits = fraction
            else:
                fraction = int(math.ldexp(mantissa, 53)) - 4503599627370496
                bits = (exponent + 1022) * 4503599627370496 + fraction
            if math.copysign(1.0, value) < 0:
                bits += 9223372036854775808
        return bits.to_bytes(8, 'little' if format == '<d' else 'big')
    layout = _int_layout(format)
    if layout is not None:
        endian, pieces, extent = layout
        if len(values) != len(pieces):
            raise error('pack expected ' + str(len(pieces)) + ' items for packing (got ' + str(len(values)) + ')')
        out = b''
        at = 0
        for (size, signed, code, place), value in zip(pieces, values):
            while at < place:
                out += b'\x00'
                at += 1
            number = _whole(value)
            try:
                out += number.to_bytes(size, endian, signed=signed)
            except OverflowError:
                raise error(_int_range(code, size, signed))
            at += size
        return out
    raise 'NotImplementedError: struct.pack needs byte values'

def unpack(format, buffer):
    if format == '<q' or format == '>q' or format == '!q':
        if len(buffer) != 8:
            raise error('unpack requires a buffer of 8 bytes')
        return (int.from_bytes(buffer, 'little' if format == '<q' else 'big', signed=True),)
    if format == '<d' or format == '>d' or format == '!d':
        if len(buffer) != 8:
            raise error('unpack requires a buffer of 8 bytes')
        import math
        bits = int.from_bytes(buffer, 'little' if format == '<d' else 'big')
        negative = bits >= 9223372036854775808
        if negative:
            bits -= 9223372036854775808
        exponent = bits // 4503599627370496
        fraction = bits % 4503599627370496
        if exponent == 2047:
            value = math.inf if fraction == 0 else math.nan
        elif exponent == 0:
            value = math.ldexp(fraction, -1074)
        else:
            value = math.ldexp(fraction + 4503599627370496, exponent - 1075)
        return (-value if negative else value,)
    if format == '<f' or format == '>f' or format == '!f':
        if len(buffer) != 4:
            raise error('unpack requires a buffer of 4 bytes')
        import math
        bits = int.from_bytes(buffer, 'little' if format == '<f' else 'big')
        negative = bits >= 2147483648
        if negative:
            bits -= 2147483648
        exponent = bits // 8388608
        fraction = bits % 8388608
        if exponent == 255:
            value = math.inf if fraction == 0 else math.nan
        elif exponent == 0:
            value = math.ldexp(fraction, -149)
        else:
            value = math.ldexp(fraction + 8388608, exponent - 150)
        return (-value if negative else value,)
    layout = _int_layout(format)
    if layout is not None:
        endian, pieces, extent = layout
        if len(buffer) != extent:
            raise error('unpack requires a buffer of ' + str(extent) + ' bytes')
        values = []
        for size, signed, code, place in pieces:
            values.append(int.from_bytes(buffer[place:place + size], endian, signed=signed))
        return tuple(values)
    raise 'NotImplementedError: struct.unpack needs byte values'

_INT_SIZES = {'b': (1, True), 'B': (1, False), 'h': (2, True), 'H': (2, False),
              'i': (4, True), 'I': (4, False), 'q': (8, True), 'Q': (8, False)}
_INT_WORDS = {'b': 'byte', 'B': 'ubyte', 'h': 'short', 'H': 'ushort'}

def _int_layout(format):
    # The pieces an integer-only format packs to, in order, each with
    # the place it stands at: (endian, [(size, signed, code, at)],
    # extent). A native prefix takes the platform's own sizes -- l and
    # L are eight bytes here -- and pads each piece to its own
    # alignment, the way the platform's compiler would lay the fields
    # out; None where a format has anything else.
    native = True
    endian = 'little'
    if format[:1] in ('@', '=', '<', '>', '!'):
        head = format[0]
        format = format[1:]
        native = head == '@'
        endian = 'little' if head in ('<', '=', '@') else 'big'
    pieces = []
    count = ''
    at = 0
    for code in format:
        if code in '0123456789':
            count += code
            continue
        if code in ' \t\n\r\v\f' and not count:
            continue
        if code == 'l' or code == 'L':
            entry = (8 if native else 4, code == 'l')
        elif code in _INT_SIZES:
            entry = _INT_SIZES[code]
        else:
            return None
        for _ in range(int(count) if count else 1):
            size, signed = entry
            if native and at % size:
                at += size - at % size
            pieces.append((size, signed, code, at))
            at += size
        count = ''
    if count:
        return None
    return (endian, pieces, at)

def _whole(value):
    # The integer index protocol: a whole number, or an object
    # answering __index__; anything else is refused the way the
    # reference refuses it, float included.
    if type(value) == type(1) or type(value) == type(True):
        return int(value)
    ask = getattr(value, '__index__', None)
    if ask is not None:
        return int(ask())
    raise error('required argument is not an integer')

def _int_range(code, size, signed):
    if code in _INT_WORDS:
        if signed:
            edge = 1 << (size * 8 - 1)
            return _INT_WORDS[code] + ' format requires ' + str(-edge) + ' <= number <= ' + str(edge - 1)
        return _INT_WORDS[code] + ' format requires 0 <= number <= ' + str((1 << (size * 8)) - 1)
    return 'argument out of range'

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
