# The part of ast that needs no parser. literal_eval is here in full: it
# reads the literal grammar -- numbers, strings, bytes, the three keyword
# constants, Ellipsis, tuples, lists, dicts, sets, a signed number and a
# number plus or minus an imaginary one -- straight from the source text,
# and accepts exactly what CPython's literal_eval accepts.
#
# Everything that needs a syntax tree does not exist here. This runtime does
# not hand its parse back to Python, so there is no node to build, walk,
# dump or unparse, and no node type to compare against; parse and its
# neighbours say so rather than return something shaped like a tree.
#
# One difference from CPython is worth naming: CPython raises SyntaxError for
# source that will not parse and ValueError for source that parses into
# something that is not a literal. Without a full parser the two cannot
# always be told apart, so source this reader cannot accept is reported as
# ValueError unless it plainly runs out of tokens.

PyCF_ONLY_AST = 1024
PyCF_TYPE_COMMENTS = 4096
PyCF_ALLOW_TOP_LEVEL_AWAIT = 8192
PyCF_OPTIMIZED_AST = 33792

_NO_TREE = 'NotImplementedError: this runtime does not expose a syntax tree'

_DIGITS = '0123456789'
_HEX = '0123456789abcdefABCDEF'
_SIMPLE_ESCAPES = {'\\': '\\', "'": "'", '"': '"', 'n': '\n', 't': '\t',
                   'r': '\r', 'a': '\a', 'b': '\b', 'f': '\f', 'v': '\v'}

def _malformed(source):
    raise 'ValueError: malformed node or string on line 1: ' + repr(source)

def _incomplete():
    raise 'SyntaxError: invalid syntax'

def _is_name_start(c):
    return c == '_' or c.isalpha()

def _is_name_char(c):
    return c == '_' or c.isalnum()

def _tokenize(source):
    # Produces (kind, text) pairs. Kinds are 'num', 'str', 'bytes', 'name'
    # and 'op'; string and bytes tokens carry their decoded value.
    tokens = []
    i = 0
    n = len(source)
    while i < n:
        c = source[i]
        if c == ' ' or c == '\t' or c == '\n' or c == '\r' or c == '\f' or c == '\v':
            i += 1
            continue
        if c == '\\' and i + 1 < n and source[i + 1] == '\n':
            i += 2
            continue
        if c == '#':
            while i < n and source[i] != '\n':
                i += 1
            continue
        if c in _DIGITS or (c == '.' and i + 1 < n and source[i + 1] in _DIGITS):
            start = i
            while i < n and (source[i] in _DIGITS or source[i] in 'abcdefABCDEFxXoO_.jJ'):
                if source[i] in 'eE' :
                    i += 1
                    if i < n and (source[i] == '+' or source[i] == '-'):
                        i += 1
                    continue
                i += 1
            tokens.append(('num', source[start:i]))
            continue
        if _is_name_start(c):
            start = i
            while i < n and _is_name_char(source[i]):
                i += 1
            word = source[start:i]
            lowered = word.lower()
            if i < n and (source[i] == "'" or source[i] == '"') and _is_prefix(lowered):
                value, i = _read_string(source, i, lowered)
                if 'b' in lowered:
                    tokens.append(('bytes', value))
                else:
                    tokens.append(('str', value))
                continue
            tokens.append(('name', word))
            continue
        if c == "'" or c == '"':
            value, i = _read_string(source, i, '')
            tokens.append(('str', value))
            continue
        if c == '.' and source[i:i + 3] == '...':
            tokens.append(('op', '...'))
            i += 3
            continue
        if c in '()[]{},:+-':
            tokens.append(('op', c))
            i += 1
            continue
        _malformed(source)
    return tokens

def _is_prefix(lowered):
    return lowered in ('r', 'b', 'u', 'rb', 'br')

def _read_string(source, i, prefix):
    # i points at the opening quote. Returns the decoded value and the
    # index just past the closing quote.
    n = len(source)
    quote = source[i]
    if source[i:i + 3] == quote * 3:
        closer = quote * 3
        i += 3
    else:
        closer = quote
        i += 1
    raw = 'r' in prefix
    is_bytes = 'b' in prefix
    units = []
    while True:
        if i >= n:
            _incomplete()
        if source[i:i + len(closer)] == closer:
            i += len(closer)
            break
        c = source[i]
        if c == '\\' and i + 1 < n:
            if raw:
                units.append('\\')
                units.append(source[i + 1])
                i += 2
                continue
            piece, i = _read_escape(source, i + 1, is_bytes)
            if piece is not None:
                units.append(piece)
            continue
        units.append(c)
        i += 1
    if is_bytes:
        numbers = []
        for unit in units:
            point = ord(unit)
            if point > 255:
                raise 'SyntaxError: bytes can only contain ASCII literal characters'
            numbers.append(point)
        return (bytes(numbers), i)
    return (''.join(units), i)

def _read_escape(source, i, is_bytes):
    # i points just past the backslash. Returns the replacement text (or
    # None for a continued line) and the index past the escape.
    n = len(source)
    c = source[i]
    if c == '\n':
        return (None, i + 1)
    if c in _SIMPLE_ESCAPES:
        return (_SIMPLE_ESCAPES[c], i + 1)
    if c >= '0' and c <= '7':
        digits = ''
        while i < n and len(digits) < 3 and source[i] >= '0' and source[i] <= '7':
            digits += source[i]
            i += 1
        return (chr(int(digits, 8)), i)
    if c == 'x':
        return _read_hex(source, i + 1, 2, '\\x')
    if c == 'u':
        if is_bytes:
            return ('\\u', i + 1)
        return _read_hex(source, i + 1, 4, '\\u')
    if c == 'U':
        if is_bytes:
            return ('\\U', i + 1)
        return _read_hex(source, i + 1, 8, '\\U')
    if c == 'N' and not is_bytes:
        if source[i + 1:i + 2] != '{':
            raise 'SyntaxError: malformed \\N character escape'
        close = i + 2
        while close < n and source[close] != '}':
            close += 1
        if close >= n:
            raise 'SyntaxError: malformed \\N character escape'
        import unicodedata
        return (unicodedata.lookup(source[i + 2:close]), close + 1)
    # An unknown escape keeps the backslash, as Python does.
    return ('\\' + c, i + 1)

def _read_hex(source, i, width, label):
    digits = source[i:i + width]
    if len(digits) < width:
        raise 'SyntaxError: truncated ' + label + ' escape'
    for digit in digits:
        if digit not in _HEX:
            raise 'SyntaxError: truncated ' + label + ' escape'
    return (chr(int(digits, 16)), i + width)

def _number(text, source):
    body = text.replace('_', '')
    if body[-1:] == 'j' or body[-1:] == 'J':
        return complex(0, _real(body[:-1], source))
    lowered = body.lower()
    if lowered[:2] == '0x':
        return _based(body[2:], 16, source)
    if lowered[:2] == '0o':
        return _based(body[2:], 8, source)
    if lowered[:2] == '0b':
        return _based(body[2:], 2, source)
    return _real(body, source)

def _based(digits, base, source):
    if digits == '':
        _malformed(source)
    try:
        return int(digits, base)
    except Exception:
        _malformed(source)

def _real(body, source):
    if body == '':
        _malformed(source)
    floating = False
    for c in body:
        if c == '.' or c == 'e' or c == 'E':
            floating = True
    try:
        if floating:
            return float(body)
        return int(body)
    except Exception:
        _malformed(source)

class _Reader:
    def __init__(self, source, tokens):
        self.source = source
        self.tokens = tokens
        self.at = 0

    def peek(self):
        if self.at >= len(self.tokens):
            return (None, None)
        return self.tokens[self.at]

    def take(self):
        if self.at >= len(self.tokens):
            _incomplete()
        token = self.tokens[self.at]
        self.at += 1
        return token

    def at_op(self, text):
        kind, value = self.peek()
        return kind == 'op' and value == text

    def expect(self, text):
        if not self.at_op(text):
            if self.at >= len(self.tokens):
                _incomplete()
            _malformed(self.source)
        self.at += 1

    def top(self):
        # The top level allows a bare tuple: 1, 2
        first = self.element()
        if not self.at_op(','):
            if self.at < len(self.tokens):
                _malformed(self.source)
            return first
        items = [first]
        while self.at_op(','):
            self.at += 1
            if self.at >= len(self.tokens):
                break
            items.append(self.element())
        if self.at < len(self.tokens):
            _malformed(self.source)
        return tuple(items)

    def element(self):
        # A signed number, or a number plus or minus an imaginary number.
        left = self.signed()
        while self.at_op('+') or self.at_op('-'):
            kind, sign = self.take()
            right = self.number_only()
            if type(right) != type(1j):
                _malformed(self.source)
            if type(left) != type(1) and type(left) != type(1.0):
                _malformed(self.source)
            left = left + right if sign == '+' else left - right
        return left

    def signed(self):
        if self.at_op('+') or self.at_op('-'):
            kind, sign = self.take()
            value = self.number_only()
            return value if sign == '+' else -value
        return self.atom()

    def number_only(self):
        kind, text = self.take()
        if kind != 'num':
            _malformed(self.source)
        return _number(text, self.source)

    def atom(self):
        kind, text = self.peek()
        if kind is None:
            _incomplete()
        if kind == 'num':
            self.at += 1
            return _number(text, self.source)
        if kind == 'str':
            return self.joined('str', '')
        if kind == 'bytes':
            return self.joined('bytes', bytes())
        if kind == 'name':
            self.at += 1
            if text == 'None':
                return None
            if text == 'True':
                return True
            if text == 'False':
                return False
            if text == 'set' and self.at_op('('):
                self.at += 1
                self.expect(')')
                return set()
            _malformed(self.source)
        if text == '...':
            self.at += 1
            return Ellipsis
        if text == '(':
            self.at += 1
            return self.round()
        if text == '[':
            self.at += 1
            return self.square()
        if text == '{':
            self.at += 1
            return self.curly()
        _malformed(self.source)

    def joined(self, kind, empty):
        # Adjacent string or bytes literals concatenate, as in source.
        value = empty
        while True:
            following, text = self.peek()
            if following != kind:
                break
            self.at += 1
            value = value + text
        return value

    def round(self):
        if self.at_op(')'):
            self.at += 1
            return tuple()
        first = self.element()
        if self.at_op(')'):
            self.at += 1
            return first
        items = [first]
        trailing = False
        while self.at_op(','):
            self.at += 1
            if self.at_op(')'):
                trailing = True
                break
            items.append(self.element())
        self.expect(')')
        return tuple(items)

    def square(self):
        items = []
        while not self.at_op(']'):
            items.append(self.element())
            if not self.at_op(','):
                break
            self.at += 1
        self.expect(']')
        return items

    def curly(self):
        if self.at_op('}'):
            self.at += 1
            return {}
        first = self.element()
        if self.at_op(':'):
            self.at += 1
            mapping = {first: self.element()}
            while self.at_op(','):
                self.at += 1
                if self.at_op('}'):
                    break
                key = self.element()
                self.expect(':')
                mapping[key] = self.element()
            self.expect('}')
            return mapping
        members = set()
        members.add(first)
        while self.at_op(','):
            self.at += 1
            if self.at_op('}'):
                break
            members.add(self.element())
        self.expect('}')
        return members

def literal_eval(node_or_string):
    if type(node_or_string) == type(b''):
        raise 'NotImplementedError: ast.literal_eval needs text, not bytes'
    if type(node_or_string) != type(''):
        _malformed(node_or_string)
    tokens = _tokenize(node_or_string)
    if not tokens:
        _incomplete()
    return _Reader(node_or_string, tokens).top()

def parse(source, filename='<unknown>', mode='exec', *, type_comments=False,
          feature_version=None, optimize=-1):
    raise _NO_TREE

def unparse(ast_obj):
    raise _NO_TREE

def dump(node, annotate_fields=True, include_attributes=False, *, indent=None):
    raise _NO_TREE

def walk(node):
    raise _NO_TREE

def iter_fields(node):
    raise _NO_TREE

def iter_child_nodes(node):
    raise _NO_TREE

def get_docstring(node, clean=True):
    raise _NO_TREE

def fix_missing_locations(node):
    raise _NO_TREE

def increment_lineno(node, n=1):
    raise _NO_TREE

def copy_location(new_node, old_node):
    raise _NO_TREE

def get_source_segment(source, node, *, padded=False):
    raise _NO_TREE

def compare(a, b, *, compare_attributes=False):
    raise _NO_TREE

class NodeVisitor:
    def visit(self, node):
        raise _NO_TREE

class NodeTransformer(NodeVisitor):
    pass
