"""JSON scanner and encoder objects implementing CPython's _json contract.

The string and container loops here are independent of json's fallback
functions. Container callbacks and hooks retain the decoder's public API.
"""
import re
import sys

_string_scan = __json_string_scan

_escapes = {'"': '"', '\\': '\\', '/': '/', 'b': '\b', 'f': '\f', 'n': '\n', 'r': '\r', 't': '\t'}
_quotes = {'"': '\\"', '\\': '\\\\', '\b': '\\b', '\f': '\\f', '\n': '\\n', '\r': '\\r', '\t': '\\t'}
_number = re.compile(r'(-?(?:0|[1-9][0-9]*))(\.[0-9]+)?([eE][-+]?[0-9]+)?')


def _index(value):
    import operator
    try:
        return operator.index(value)
    except TypeError:
        if getattr(type(value), '__index__', None) is None:
            raise TypeError("'" + type(value).__name__ + "' object cannot be interpreted as an integer") from None
        raise


def _error(message, text, position):
    from json import JSONDecodeError
    raise JSONDecodeError(message, text, position)


def _unicode_escape(text, position):
    digits = text[position:position + 4]
    if len(digits) != 4 or any(c not in '0123456789abcdefABCDEF' for c in digits):
        _error('Invalid \\uXXXX escape', text, position - 1)
    return int(digits, 16)


def scanstring(string, end, strict=True):
    if not isinstance(string, str):
        raise TypeError('first argument must be a string')
    if type(end) is not int:
        end = _index(end)
    if end > sys.maxsize or end < -sys.maxsize - 1:
        raise OverflowError('Python int too large to convert to C ssize_t')
    strict = bool(strict)
    result = __json_string_scan(string, end)
    if result is not None:
        return result
    begin = end - 1
    pieces = []
    while 0 <= end < len(string):
        character = string[end]
        end += 1
        if character == '"':
            return ''.join(pieces), end
        if character == '\\':
            if end == len(string):
                break
            escape = string[end]
            end += 1
            if escape == 'u':
                code = _unicode_escape(string, end)
                end += 4
                if 0xd800 <= code <= 0xdbff and string[end:end + 2] == '\\u':
                    low = _unicode_escape(string, end + 2)
                    if 0xdc00 <= low <= 0xdfff:
                        code = 0x10000 + ((code - 0xd800) << 10) + low - 0xdc00
                        end += 6
                character = chr(code)
            elif escape in _escapes:
                character = _escapes[escape]
            else:
                _error('Invalid \\escape', string, end - 2)
        elif ord(character) < 32 and strict:
            _error('Invalid control character at', string, end - 1)
        pieces.append(character)
    _error('Unterminated string starting at', string, begin)


def _quote(string, ascii_only):
    if not isinstance(string, str):
        raise TypeError('first argument must be a string')
    encoded = _string_scan(string, ascii_only, True)
    if encoded is not None:
        return encoded
    parts = ['"']
    for character in string:
        if character in _quotes:
            parts.append(_quotes[character])
        else:
            code = ord(character)
            if code < 32 or (ascii_only and code > 126):
                if code > 0xffff:
                    code -= 0x10000
                    parts.append('\\u%04x\\u%04x' % (0xd800 + (code >> 10), 0xdc00 + (code & 1023)))
                else:
                    parts.append('\\u%04x' % code)
            else:
                parts.append(character)
    parts.append('"')
    return ''.join(parts)


def encode_basestring(string):
    return _quote(string, False)


def encode_basestring_ascii(string):
    return _quote(string, True)


class Scanner:
    def __init__(self, context):
        self.strict = bool(context.strict)
        self.parse_float = context.parse_float
        self.parse_int = context.parse_int
        self.parse_constant = context.parse_constant
        self.object_hook = context.object_hook
        self.object_pairs_hook = context.object_pairs_hook
        self.parse_object = context.parse_object
        self.parse_array = context.parse_array
        self.memo = context.memo

    def __call__(self, string, index):
        if not isinstance(string, str):
            raise TypeError('first argument must be a string')
        if type(index) is not int:
            index = _index(index)
        if index > sys.maxsize or index < -sys.maxsize - 1:
            raise OverflowError('Python int too large to convert to C ssize_t')
        if index < 0:
            raise ValueError('idx cannot be negative')
        try:
            if index < len(string) and string[index] == '"':
                scanned = _string_scan(string, index + 1)
                if scanned is not None:
                    return scanned
            return self._scan(string, index)
        finally:
            self.memo.clear()

    def _scan(self, string, index):
        if index >= len(string):
            raise StopIteration(index)
        char = string[index]
        if char == '"':
            return scanstring(string, index + 1, self.strict)
        if char == '{':
            return self.parse_object((string, index + 1), self.strict, self._scan,
                                     self.object_hook, self.object_pairs_hook, self.memo)
        if char == '[':
            return self.parse_array((string, index + 1), self._scan)
        for literal, value in [('null', None), ('true', True), ('false', False)]:
            if string[index:index + len(literal)] == literal:
                return value, index + len(literal)
        match = _number.match(string, index)
        if match is not None:
            value = match.group()
            convert = self.parse_float if '.' in value or 'e' in value or 'E' in value else self.parse_int
            return convert(value), match.end()
        for literal in ['NaN', 'Infinity', '-Infinity']:
            if string[index:index + len(literal)] == literal:
                return self.parse_constant(literal), index + len(literal)
        raise StopIteration(index)


make_scanner = Scanner


class Encoder:
    def __init__(self, markers, default, encoder, indent, key_separator,
                 item_separator, sort_keys, skipkeys, allow_nan):
        if markers is not None and not isinstance(markers, dict):
            raise TypeError('make_encoder() argument 1 must be dict or None, not ' + type(markers).__name__)
        self.markers = markers
        self.default = default
        self.encoder = encoder
        self.indent = indent
        self.key_separator = key_separator
        self.item_separator = item_separator
        self.sort_keys = bool(sort_keys)
        self.skipkeys = bool(skipkeys)
        self.allow_nan = bool(allow_nan)

    def __call__(self, value, current_indent_level):
        if not isinstance(current_indent_level, int):
            raise TypeError('an integer is required')
        return (self._encode(value, max(0, current_indent_level)),)

    def _string(self, value):
        result = self.encoder(value)
        if not isinstance(result, str):
            raise TypeError('encoder must return a string, not ' + type(result).__name__)
        return result

    def _float(self, value):
        if value != value:
            text = 'NaN'
        elif value == float('inf'):
            text = 'Infinity'
        elif value == -float('inf'):
            text = '-Infinity'
        else:
            return float.__repr__(value)
        if not self.allow_nan:
            raise ValueError('Out of range float values are not JSON compliant: ' + repr(value))
        return text

    def _encode(self, value, level):
        if isinstance(value, str):
            return self._string(value)
        if value is None:
            return 'null'
        if value is True:
            return 'true'
        if value is False:
            return 'false'
        if isinstance(value, int):
            return int.__repr__(value)
        if isinstance(value, float):
            return self._float(value)
        marker = id(value)
        if self.markers is not None:
            if marker in self.markers:
                raise ValueError('Circular reference detected')
            self.markers[marker] = value
        try:
            if isinstance(value, (list, tuple)):
                if not value:
                    return '[]'
                parts = []
                index = 0
                while index < len(value):
                    item = value[index]
                    index += 1
                    try:
                        parts.append(self._encode(item, level + 1))
                    except GeneratorExit:
                        raise
                    except BaseException as error:
                        error.add_note(f'when serializing {type(value).__name__} item {index - 1}')
                        raise
                return self._container(parts, '[', ']', level)
            if isinstance(value, dict):
                items = value.items()
                if self.sort_keys:
                    items = sorted(items)
                parts = []
                for key, item in items:
                    original_key = key
                    if isinstance(key, str):
                        pass
                    elif key is True:
                        key = 'true'
                    elif key is False:
                        key = 'false'
                    elif key is None:
                        key = 'null'
                    elif isinstance(key, int):
                        key = int.__repr__(key)
                    elif isinstance(key, float):
                        key = self._float(key)
                    elif self.skipkeys:
                        continue
                    else:
                        raise TypeError('keys must be str, int, float, bool or None, not ' + type(key).__name__)
                    encoded_key = self._string(key)
                    try:
                        parts.append(encoded_key + self.key_separator + self._encode(item, level + 1))
                    except GeneratorExit:
                        raise
                    except BaseException as error:
                        error.add_note(f'when serializing {type(value).__name__} item {original_key!r}')
                        raise
                return self._container(parts, '{', '}', level)
            converted = self.default(value)
            try:
                return self._encode(converted, level)
            except GeneratorExit:
                raise
            except BaseException as error:
                error.add_note(f'when serializing {type(value).__name__} object')
                raise
        finally:
            if self.markers is not None:
                del self.markers[marker]

    def _container(self, parts, left, right, level):
        if not parts:
            return left + right
        if self.indent is None:
            return left + self.item_separator.join(parts) + right
        child_indent = '\n' + self.indent * (level + 1)
        return left + child_indent + (self.item_separator + child_indent).join(parts) + '\n' + self.indent * level + right


make_encoder = Encoder
