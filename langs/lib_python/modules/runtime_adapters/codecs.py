# Extend the codec registry with CPython-derived GBK and GB2312 mapping data.
_LegacyTables = {}
_LegacyEncoders = {}

# Load the mapping rows only when a codec from this family is requested.
def _legacy_rows(name):
    if name not in _LegacyTables:
        import _legacy_charmaps
        rows = _legacy_charmaps._GBK_ROWS if name == 'gbk' else _legacy_charmaps._GB2312_ROWS
        _LegacyTables[name] = rows
    return _LegacyTables[name]

# Index the canonical two-byte encodings of every mapped character.
def _legacy_reverse(name):
    if name not in _LegacyEncoders:
        found = {}
        first = 0x81 if name == 'gbk' else 0xa1
        for offset, row in enumerate(_legacy_rows(name)):
            for column, character in enumerate(row):
                if character == '\0':
                    continue
                second = column + 0x40 + (1 if column >= 0x3f else 0) if name == 'gbk' else column + 0xa1
                if character not in found:
                    found[character] = bytes((first + offset, second))
        _LegacyEncoders[name] = found
    return _LegacyEncoders[name]

# Encode text and honor the existing error-handler registry and positions.
def _legacy_encode(text, name, errors):
    if not isinstance(text, str):
        raise TypeError('encode() argument 1 must be str, not ' + type(text).__name__)
    result = bytearray()
    position = 0
    reverse = None
    while position < len(text):
        character = text[position]
        if ord(character) < 128:
            result.append(ord(character))
            position += 1
            continue
        if reverse is None:
            reverse = _legacy_reverse(name)
        encoded = reverse.get(character)
        if encoded is not None:
            result.extend(encoded)
            position += 1
            continue
        replacement, position = _encode_error(name, text, position, position + 1, 'illegal multibyte sequence', errors)
        result.extend(replacement if isinstance(replacement, bytes) else _legacy_encode(replacement, name, 'strict'))
    return bytes(result)

# Decode complete sequences and leave an incomplete tail for incremental callers.
def _legacy_decode(data, name, errors, final=True):
    if not isinstance(data, (bytes, bytearray, memoryview)):
        raise TypeError('a bytes-like object is required, not ' + type(data).__name__)
    data = bytes(data)
    rows = _legacy_rows(name)
    first = 0x81 if name == 'gbk' else 0xa1
    result = []
    position = 0
    while position < len(data):
        lead = data[position]
        if lead < 128:
            result.append(chr(lead))
            position += 1
            continue
        valid_lead = first <= lead <= 0xfe
        if position + 1 == len(data) and not final:
            break
        character = None
        if valid_lead and position + 1 < len(data):
            trail = data[position + 1]
            valid_trail = 0x40 <= trail <= 0xfe and trail != 0x7f if name == 'gbk' else 0xa1 <= trail <= 0xfe
            if valid_trail:
                column = trail - 0x40 - (1 if trail > 0x7f else 0) if name == 'gbk' else trail - 0xa1
                character = rows[lead - first][column]
        if character is not None and character != '\0':
            result.append(character)
            position += 2
            continue
        reason = 'incomplete multibyte sequence' if position + 1 == len(data) else 'illegal multibyte sequence'
        replacement, position = _decode_error(name, data, position, position + 1, reason, errors)
        result.append(replacement)
    return ''.join(result), position

class _LegacyEncoder(IncrementalEncoder):
    # Retain the selected codec and the registered error policy.
    def __init__(self, name, errors='strict'):
        self.name = name
        self.errors = errors

    # Encode each chunk; these encodings have no shift state.
    def encode(self, text, final=False):
        return _legacy_encode(text, self.name, self.errors)

class _LegacyDecoder(IncrementalDecoder):
    # Keep incomplete input independently of the mapping data.
    def __init__(self, name, errors='strict'):
        self.name = name
        self.errors = errors
        self.buffer = b''

    # Decode the available complete characters and retain the trailing lead byte.
    def decode(self, data, final=False):
        joined = self.buffer + bytes(data)
        output, consumed = _legacy_decode(joined, self.name, self.errors, final)
        self.buffer = joined[consumed:]
        return output

    # Discard the incomplete sequence when the caller resets the decoder.
    def reset(self):
        self.buffer = b''

    # Expose the actual buffered bytes and the stateless codec state.
    def getstate(self):
        return self.buffer, 0

    # Restore the buffered bytes of this stateless decoder.
    def setstate(self, state):
        self.buffer = bytes(state[0])

# Resolve standard aliases and supply genuine codec implementations to lookup().
def _legacy_search(encoding):
    from encodings.aliases import aliases
    name = aliases.get(encoding, encoding)
    if name not in ('gbk', 'gb2312'):
        return None
    return CodecInfo(encode=lambda text, errors='strict': (_legacy_encode(text, name, errors), len(text)),
                     decode=lambda data, errors='strict': _legacy_decode(data, name, errors),
                     incrementalencoder=lambda errors='strict': _LegacyEncoder(name, errors),
                     incrementaldecoder=lambda errors='strict': _LegacyDecoder(name, errors),
                     name=name)

register(_legacy_search)
