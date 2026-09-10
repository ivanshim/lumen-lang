# Text is read as JSON, never as executable source.
class JSONDecodeError(ValueError):
    def __init__(self, msg, doc, pos):
        self.msg = msg
        self.doc = doc
        self.pos = pos
        self.lineno = 1
        self.colno = 1
        for letter in list(doc[:pos]):
            if letter == '\n':
                self.lineno += 1
                self.colno = 1
            else:
                self.colno += 1
        self.message = self.__str__()

    def __str__(self):
        return self.msg + ': line ' + str(self.lineno) + ' column ' + str(self.colno) + ' (char ' + str(self.pos) + ')'


def _quote(text, ascii):
    result = '"'
    digits = '0123456789abcdef'
    for c in list(text):
        n = ord(c)
        if c == '"' or c == '\\':
            result += '\\' + c
        elif c == '\n':
            result += '\\n'
        elif c == '\r':
            result += '\\r'
        elif c == '\t':
            result += '\\t'
        elif c == '\b':
            result += '\\b'
        elif c == '\f':
            result += '\\f'
        elif n < 32 or (ascii and n > 127):
            if n > 65535:
                n -= 65536
                high = 55296 + n // 1024
                result += '\\u' + digits[high // 4096] + digits[high // 256 % 16] + digits[high // 16 % 16] + digits[high % 16]
                n = 56320 + n % 1024
            result += '\\u' + digits[n // 4096] + digits[n // 256 % 16] + digits[n // 16 % 16] + digits[n % 16]
        else:
            result += c
    return result + '"'


def dumps(obj, *, skipkeys=False, ensure_ascii=True, check_circular=True, allow_nan=True, cls=None, indent=None, separators=None, default=None, sort_keys=False, **kw):
    if cls is not None or len(kw) != 0:
        raise 'NotImplementedError: json.dumps custom encoders are not supported'
    gap = None
    if indent is not None and not isinstance(indent, type('')) and not isinstance(indent, type(1)) and not isinstance(indent, type(True)):
        raise 'TypeError: JSON indentation must be an integer or a string'
    if indent is not None:
        gap = indent if type(indent) == type('') else _repeat(' ', indent)
    comma = ', ' if gap is None else ','
    colon = ': '
    if separators is not None:
        if len(separators) != 2:
            raise 'ValueError: JSON separators need two values'
        if not isinstance(separators[0], type('')) or not isinstance(separators[1], type('')):
            raise 'TypeError: JSON separators must be strings'
        comma = separators[0]
        colon = separators[1]
    return _encode(obj, gap, comma, colon, ensure_ascii, sort_keys, default, allow_nan, skipkeys, 0)


def _encode(obj, gap, comma, colon, ascii, ordered, default, allow_nan, skipkeys, depth):
    if depth > 100:
        raise 'ValueError: JSON nesting exceeds the supported depth'
    if obj is None:
        return 'null'
    if isinstance(obj, type(True)):
        return 'true' if obj else 'false'
    if isinstance(obj, type('')):
        return _quote(obj, ascii)
    if isinstance(obj, type(1)) or isinstance(obj, type(1.0)):
        text = str(obj)
        if text == 'nan' or text == 'inf' or text == '-inf':
            if not allow_nan:
                raise 'ValueError: Out of range float values are not JSON compliant'
            if text == 'nan':
                return 'NaN'
            return '-Infinity' if text == '-inf' else 'Infinity'
        if isinstance(obj, type(1.0)) and '.' not in text and 'e' not in text and 'E' not in text:
            text += '.0'
        return text
    if __is_mapping(obj):
        keys = list(obj)
        if ordered:
            # An insertion sort needs no host sorting convention.
            for i in range(1, len(keys)):
                key = keys[i]
                j = i
                while j > 0 and _greater(keys[j - 1], key):
                    keys[j] = keys[j - 1]
                    j -= 1
                keys[j] = key
        pieces = []
        for key in keys:
            if isinstance(key, type('')):
                word = key
            elif key is None:
                word = 'null'
            elif isinstance(key, type(True)):
                word = 'true' if key else 'false'
            elif isinstance(key, type(1)) or isinstance(key, type(1.0)):
                word = _encode(key, None, comma, colon, ascii, False, None, allow_nan, False, depth + 1)
            elif skipkeys:
                continue
            else:
                raise 'TypeError: JSON keys must be strings, numbers, booleans or None'
            pieces.append(_quote(word, ascii) + colon + _encode(obj[key], gap, comma, colon, ascii, ordered, default, allow_nan, skipkeys, depth + 1))
        opening = '{'
        closing = '}'
    elif isinstance(obj, type([])) or isinstance(obj, type(())):
        pieces = []
        for item in obj:
            pieces.append(_encode(item, gap, comma, colon, ascii, ordered, default, allow_nan, skipkeys, depth + 1))
        opening = '['
        closing = ']'
    elif default is not None:
        return _encode(default(obj), gap, comma, colon, ascii, ordered, default, allow_nan, skipkeys, depth + 1)
    else:
        raise 'TypeError: Object is not JSON serializable'
    if len(pieces) == 0:
        return opening + closing
    separator = comma
    if gap is not None:
        opening += '\n' + _repeat(gap, depth + 1)
        separator += '\n' + _repeat(gap, depth + 1)
        closing = '\n' + _repeat(gap, depth) + closing
    result = opening + pieces[0]
    for part in pieces[1:]:
        result += separator + part
    return result + closing


class _Reader:
    def __init__(self, text):
        self.text = text
        self.at = 0

    def bad(self, message):
        raise JSONDecodeError(message, self.text, self.at)

    def space(self):
        while self.at < len(self.text) and self.text[self.at] in ' \t\r\n':
            self.at += 1

    def string(self):
        self.at += 1
        result = ''
        while self.at < len(self.text):
            c = self.text[self.at]
            self.at += 1
            if c == '"':
                return result
            if c == '\\':
                if self.at == len(self.text):
                    self.bad('Unterminated string')
                c = self.text[self.at]
                self.at += 1
                escapes = {'"': '"', '\\': '\\', '/': '/', 'b': '\b', 'f': '\f', 'n': '\n', 'r': '\r', 't': '\t'}
                if c in escapes:
                    result += escapes[c]
                elif c == 'u':
                    number = self.hex4()
                    if number >= 55296 and number <= 56319:
                        if self.text[self.at:self.at + 2] == '\\u':
                            saved = self.at
                            self.at += 2
                            low = self.hex4()
                            if low >= 56320 and low <= 57343:
                                number = 65536 + (number - 55296) * 1024 + low - 56320
                            else:
                                self.at = saved
                    if number >= 55296 and number <= 57343:
                        raise 'NotImplementedError: JSON lone surrogates need surrogate text values'
                    result += chr(number)
                else:
                    self.bad('Invalid escape')
            elif ord(c) < 32:
                self.bad('Invalid control character')
            else:
                result += c
        self.bad('Unterminated string')

    def hex4(self):
        value = 0
        for i in range(4):
            if self.at >= len(self.text):
                self.bad('Invalid Unicode escape')
            c = self.text[self.at]
            if c in 'ABCDEF':
                c = chr(ord(c) + 32)
            self.at += 1
            digit = 0
            while digit < 16 and '0123456789abcdef'[digit] != c:
                digit += 1
            if digit == 16:
                self.bad('Invalid Unicode escape')
            value = value * 16 + digit
        return value

    def value(self, depth=0):
        if depth > 100:
            self.bad('JSON nesting exceeds the supported depth')
        self.space()
        if self.at == len(self.text):
            self.bad('Expecting value')
        c = self.text[self.at]
        if c == '"':
            return self.string()
        if c == '[' or c == '{':
            mapping = c == '{'
            end = '}' if mapping else ']'
            result = {} if mapping else []
            self.at += 1
            self.space()
            if self.text[self.at:self.at + 1] == end:
                self.at += 1
                return result
            while True:
                if mapping:
                    self.space()
                    if self.text[self.at:self.at + 1] != '"':
                        self.bad('Expecting property name enclosed in double quotes')
                    key = self.string()
                    self.space()
                    if self.text[self.at:self.at + 1] != ':':
                        self.bad('Expecting colon delimiter')
                    self.at += 1
                    result[key] = self.value(depth + 1)
                else:
                    result.append(self.value(depth + 1))
                self.space()
                c = self.text[self.at:self.at + 1]
                self.at += 1
                if c == end:
                    return result
                if c != ',':
                    self.bad('Expecting comma delimiter')
        for special in ['NaN', 'Infinity', '-Infinity']:
            if self.text[self.at:self.at + len(special)] == special:
                self.at += len(special)
                if special == 'NaN':
                    return __math('fdiv', 0.0, 0.0)
                return __math('fdiv', -1.0 if special == '-Infinity' else 1.0, 0.0)
        for word, answer in [('null', None), ('true', True), ('false', False)]:
            if self.text[self.at:self.at + len(word)] == word:
                self.at += len(word)
                return answer
        start = self.at
        if c == '-':
            self.at += 1
        if self.text[self.at:self.at + 1] == '0':
            self.at += 1
        else:
            first = self.at
            while self.at < len(self.text) and self.text[self.at] in '0123456789':
                self.at += 1
            if first == self.at:
                self.bad('Expecting value')
        real = False
        if self.text[self.at:self.at + 1] == '.':
            real = True
            self.at += 1
            first = self.at
            while self.at < len(self.text) and self.text[self.at] in '0123456789':
                self.at += 1
            if first == self.at:
                self.bad('Expecting fraction digits')
        if self.at < len(self.text) and self.text[self.at] in 'eE':
            real = True
            self.at += 1
            if self.at < len(self.text) and self.text[self.at] in '+-':
                self.at += 1
            first = self.at
            while self.at < len(self.text) and self.text[self.at] in '0123456789':
                self.at += 1
            if first == self.at:
                self.bad('Expecting exponent digits')
        text = self.text[start:self.at]
        if not real:
            return int(text)
        number = float(text)
        if number == 0 and text[:1] == '-':
            return __math('fdiv', -0.0, 1.0)
        return number


def loads(s, *, cls=None, object_hook=None, parse_float=None, parse_int=None, parse_constant=None, object_pairs_hook=None, **kw):
    if cls is not None or object_hook is not None or parse_float is not None or parse_int is not None or parse_constant is not None or object_pairs_hook is not None or len(kw) != 0:
        raise 'NotImplementedError: json.loads custom decoders are not supported'
    if type(s) != type(''):
        raise 'NotImplementedError: json.loads needs text until byte values are carried'
    reader = _Reader(s)
    value = reader.value()
    reader.space()
    if reader.at != len(s):
        reader.bad('Extra data')
    return value


def dump(obj, fp, **kw):
    fp.write(dumps(obj, **kw))


def load(fp, **kw):
    return loads(fp.read(), **kw)


def _repeat(text, count):
    result = ''
    while count > 0:
        result += text
        count -= 1
    return result


def _greater(left, right):
    if isinstance(left, type('')) and isinstance(right, type('')):
        at = 0
        while at < len(left) and at < len(right):
            a = ord(left[at])
            b = ord(right[at])
            if a != b:
                return a > b
            at += 1
        return len(left) > len(right)
    if isinstance(left, type('')) or isinstance(right, type('')) or left is None or right is None:
        raise 'TypeError: JSON keys cannot be compared'
    return left > right
