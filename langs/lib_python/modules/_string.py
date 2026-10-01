# The two helpers CPython keeps behind str.format, written out in Python.
# Both walk the format string exactly as the C markup iterator does, so the
# tuples they hand back and the errors they raise match character for
# character.

# Behaviour follows CPython 3b564385e4c9, Objects/stringlib/unicode_format.h
# (PSF License); the public helpers have the same single-argument C call contract.
def _single_argument(args, keywords, name):
    if keywords:
        raise TypeError('_string.' + name + '() takes no keyword arguments')
    if len(args) != 1:
        raise TypeError('_string.' + name + '() takes exactly one argument ('
                        + str(len(args)) + ' given)')
    return args[0]

def _require_str(value, name):
    if not isinstance(value, str):
        raise TypeError('expected str, got ' + type(value).__name__)

def _parse_field(text, start):
    # Reads one replacement field beginning just after its '{'. Returns the
    # field name, the format spec, the conversion character and the index
    # just past the closing '}'.
    n = len(text)
    i = start
    last = ''
    while i < n:
        last = text[i]
        i += 1
        if last == '{':
            raise "ValueError: unexpected '{' in field name"
        if last == '[':
            while i < n and text[i] != ']':
                i += 1
            continue
        if last == '}' or last == ':' or last == '!':
            break
    field_name = text[start:i - 1]
    conversion = None
    format_spec = ''

    if last != '!' and last != ':':
        if last != '}':
            raise "ValueError: expected '}' before end of string"
        return (field_name, format_spec, conversion, i)

    if last == '!':
        if i >= n:
            raise 'ValueError: end of string while looking for conversion specifier'
        conversion = text[i]
        i += 1
        if i >= n:
            raise "ValueError: unmatched '{' in format spec"
        last = text[i]
        i += 1
        if last == '}':
            return (field_name, format_spec, conversion, i)
        if last != ':':
            raise "ValueError: expected ':' after conversion specifier"

    spec_start = i
    depth = 1
    while i < n:
        last = text[i]
        i += 1
        if last == '{':
            depth += 1
        elif last == '}':
            depth -= 1
            if depth == 0:
                return (field_name, text[spec_start:i - 1], conversion, i)
    raise "ValueError: unmatched '{' in format spec"

def formatter_parser(*args, **keywords):
    format_string = _single_argument(args, keywords, 'formatter_parser')
    _require_str(format_string, 'formatter_parser')
    return _formatter_parser(format_string)

def _formatter_parser(text):
    n = len(text)
    i = 0
    while i < n:
        start = i
        last = ''
        markup = False
        while i < n:
            last = text[i]
            i += 1
            if last == '{' or last == '}':
                markup = True
                break
            last = ''
        at_end = i >= n
        length = i - start
        if last == '}' and (at_end or text[i] != '}'):
            raise "ValueError: Single '}' encountered in format string"
        if at_end and last == '{':
            raise "ValueError: Single '{' encountered in format string"
        if not at_end:
            if last == text[i]:
                # An escaped brace: the literal keeps one of the pair and no
                # field follows it.
                i += 1
                markup = False
            else:
                length -= 1
        literal = text[start:start + length]
        if not markup:
            yield (literal, None, None, None)
            continue
        field_name, format_spec, conversion, i = _parse_field(text, i)
        yield (literal, field_name, format_spec, conversion)

# Decimal digit block starts from CPython 3b564385e4c9,
# Objects/unicodetype_db.h (PSF License), including Unicode 17 additions.
_DECIMAL_ZEROES = [48, 1632, 1776, 1984, 2406, 2534, 2662, 2790, 2918, 3046, 3174, 3302, 3430, 3558, 3664, 3792, 3872, 4160, 4240, 6112, 6160, 6470, 6608, 6784, 6800, 6992, 7088, 7232, 7248, 42528, 43216, 43264, 43472, 43504, 43600, 44016, 65296, 66720, 68912, 68928, 69734, 69872, 69942, 70096, 70384, 70736, 70864, 71248, 71360, 71376, 71386, 71472, 71904, 72016, 72688, 72784, 73040, 73120, 73184, 73552, 90416, 92768, 92864, 93008, 93552, 118000, 120782, 120792, 120802, 120812, 120822, 123200, 123632, 124144, 124401, 125264, 130032]

def _as_index(name):
    if name == '':
        return None
    answer = 0
    for letter in name:
        digit = None
        code = ord(letter)
        for zero in _DECIMAL_ZEROES:
            if zero <= code < zero + 10:
                digit = code - zero
                break
        if digit is None:
            return None
        answer = answer * 10 + digit
        if answer > 9223372036854775807:
            raise ValueError('Too many decimal digits in format string')
    return answer

def formatter_field_name_split(*args, **keywords):
    field_name = _single_argument(args, keywords, 'formatter_field_name_split')
    _require_str(field_name, 'formatter_field_name_split')
    n = len(field_name)
    i = 0
    while i < n:
        letter = field_name[i]
        if letter == '.' or letter == '[':
            break
        i += 1
    first = field_name[:i]
    index = _as_index(first)
    if index is not None:
        first = index
    return (first, _field_name_rest(field_name, i))

def _field_name_rest(text, start):
    n = len(text)
    i = start
    while i < n:
        opener = text[i]
        i += 1
        if opener == '.':
            begin = i
            while i < n:
                letter = text[i]
                if letter == '.' or letter == '[':
                    break
                i += 1
            name = text[begin:i]
            if name == '':
                raise 'ValueError: Empty attribute in format string'
            yield (True, name)
        elif opener == '[':
            begin = i
            while i < n and text[i] != ']':
                i += 1
            if i >= n:
                raise "ValueError: Missing ']' in format string"
            name = text[begin:i]
            i += 1
            if name == '':
                raise 'ValueError: Empty attribute in format string'
            index = _as_index(name)
            if index is not None:
                name = index
            yield (False, name)
        else:
            raise "ValueError: Only '.' or '[' may follow ']' in format field specifier"
