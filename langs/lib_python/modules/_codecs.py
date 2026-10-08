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
            result.append(character)
            separator = False
        else:
            separator = True
    return ''.join(result)


# The indexed (charmap) codecs read a byte-like object and write text or
# read text and write bytes.  Undefined entries stand for a character the
# table cannot name; the error policy names what to do instead.
def _charmap_bytes(data):
    if isinstance(data, str):
        raise TypeError("a bytes-like object is required, not 'str'")
    if isinstance(data, (bytes, bytearray, memoryview)):
        return bytes(data)
    raise TypeError("a bytes-like object is required, not '%s'" % type(data).__name__)

def _charmap_text(text):
    if not isinstance(text, str):
        raise TypeError('charmap_encode() argument 1 must be str, not ' + type(text).__name__)
    return text

def _charmap_policy(errors, name):
    if not isinstance(errors, str):
        raise TypeError(name + '() argument 2 must be str, not ' + type(errors).__name__)
    return errors

def _charmap_answer(exc, errors, decoding):
    import codecs
    answer = codecs.lookup_error(errors)(exc)
    if not isinstance(answer, tuple) or len(answer) != 2:
        raise TypeError('decoding error handler must return (str, int) tuple' if decoding
                        else 'encoding error handler must return (str/bytes, int) tuple')
    replacement, position = answer
    if not isinstance(replacement, str) and (decoding or not isinstance(replacement, bytes)):
        raise TypeError('decoding error handler must return (str, int) tuple' if decoding
                        else 'encoding error handler must return (str/bytes, int) tuple')
    if not isinstance(position, int):
        raise TypeError('position from error handler must be an integer')
    if position < 0:
        position += len(exc.object)
    if position < 0 or position > len(exc.object):
        raise IndexError('position %d from error handler out of bounds' % position)
    return (replacement, position)

def charmap_build(mapping):
    # A 256-character table names one byte per code point.  '\ufffe' is the
    # table's own spelling for an undefined slot and is left out.
    if not isinstance(mapping, str):
        raise TypeError('charmap_build() argument must be str, not ' + type(mapping).__name__)
    return {ord(character): index for index, character in enumerate(mapping) if character != '\ufffe'}

def charmap_decode(data, errors='strict', mapping=None):
    data = _charmap_bytes(data)
    errors = _charmap_policy(errors, 'charmap_decode')
    if mapping is None:
        return (data.decode('latin-1', errors), len(data))
    result = []
    index = 0
    while index < len(data):
        try:
            character = mapping[data[index]]
        except (KeyError, IndexError):
            character = None
        if isinstance(character, int):
            if not 0 <= character <= 0x10ffff:
                raise TypeError('character mapping must be in range(0x110000)')
            character = chr(character)
        elif character is not None and not isinstance(character, str):
            raise TypeError('character mapping must return integer, None or str')
        if character is None or character == '\ufffe':
            replacement, index = _charmap_answer(
                UnicodeDecodeError('charmap', data, index, index + 1, 'character maps to <undefined>'),
                errors, True)
            result.append(replacement)
        else:
            result.append(character)
            index += 1
    return (''.join(result), len(data))

def charmap_encode(text, errors='strict', mapping=None):
    text = _charmap_text(text)
    errors = _charmap_policy(errors, 'charmap_encode')
    if mapping is None:
        return (text.encode('latin-1', errors), len(text))
    row = []
    index = 0
    while index < len(text):
        try:
            mapped = mapping[ord(text[index])]
        except (KeyError, IndexError):
            mapped = None
        if mapped is None:
            end = index + 1
            while end < len(text):
                try:
                    following = mapping[ord(text[end])]
                except (KeyError, IndexError):
                    following = None
                if following is not None:
                    break
                end += 1
            replacement, index = _charmap_answer(
                UnicodeEncodeError('charmap', text, index, end, 'character maps to <undefined>'),
                errors, False)
            row.extend(replacement if isinstance(replacement, bytes) else charmap_encode(replacement, 'strict', mapping)[0])
        elif isinstance(mapped, int):
            if not 0 <= mapped < 256:
                raise TypeError('character mapping must be in range(256)')
            row.append(mapped)
            index += 1
        elif isinstance(mapped, (bytes, bytearray)):
            row.extend(mapped)
            index += 1
        else:
            raise TypeError('character mapping must return integer, bytes or None, not ' + type(mapped).__name__)
    return (bytes(row), len(text))
