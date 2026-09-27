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
