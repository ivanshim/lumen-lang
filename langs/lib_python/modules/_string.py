# The two helpers CPython keeps behind str.format, written out in Python.
# Both walk the format string exactly as the C markup iterator does, so the
# tuples they hand back and the errors they raise match character for
# character.

def _require_str(value, name):
    if type(value) != type(''):
        raise 'TypeError: ' + name + '() argument must be str, not ' + type(value).__name__

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

def formatter_parser(format_string):
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

def _as_index(name):
    # A field or subscript made only of digits is an index, not a name.
    if name == '':
        return None
    for letter in name:
        if letter < '0' or letter > '9':
            return None
    return int(name)

def formatter_field_name_split(field_name):
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
                if letter == ']':
                    raise "ValueError: Unexpected ']' in format string"
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
