# Python 3.14 AST nodes, a source reader and the standard tree helper algorithms.
# The node schema follows CPython's ASDL; helpers and the unparser retain their
# upstream algorithms. Source outside this reader's grammar fails visibly.
# Optimized trees, complete type-comment attachment, and compiler validation
# beyond the checks below remain unsupported rather than returning invented trees.

import warnings

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

class _Chars:
    """A text read one character at a time.

    Looking at one place in a text that holds a character outside ASCII
    walks the stored bytes from the start, so a reader that visits every
    character pays for the whole text again at each step. Spreading the
    text into a list once makes each visit a constant step; a slice is
    joined back into the same text the reader expects.
    """
    __slots__ = ('text', 'items', 'ascii')

    def __init__(self, text):
        if isinstance(text, _Chars):
            self.text = text.text
            self.items = text.items
            self.ascii = text.ascii
        else:
            self.text = text
            self.ascii = text.isascii()
            self.items = None if self.ascii else list(text)

    def __len__(self):
        return len(self.text)

    def __getitem__(self, at):
        # Text that is all ASCII indexes and slices by byte directly, so the
        # character list is only needed when a non-ASCII character would make
        # a position walk the stored bytes from the start.
        if self.ascii:
            return self.text[at]
        if isinstance(at, slice):
            return ''.join(self.items[at])
        return self.items[at]

    def __repr__(self):
        return repr(self.text)

    def __str__(self):
        return self.text

    def find(self, sub, start=0, end=None):
        return self.text.find(sub, start) if end is None else self.text.find(sub, start, end)

    def rfind(self, sub, start=0, end=None):
        return self.text.rfind(sub, start) if end is None else self.text.rfind(sub, start, end)

    def count(self, sub, start=0, end=None):
        return self.text.count(sub, start) if end is None else self.text.count(sub, start, end)

    def split(self, sep=None, maxsplit=-1):
        return self.text.split(sep, maxsplit)

    def replace(self, old, new, count=-1):
        return self.text.replace(old, new, count)


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
        quote = source.find(closer, i)
        if quote < 0:
            _incomplete()
        escape = source.find('\\', i)
        if escape >= 0 and escape < quote:
            if escape > i:
                units.append(source[i:escape])
            if escape + 1 >= n:
                units.append('\\')
                i = n
                continue
            if raw:
                units.append('\\')
                units.append(source[escape + 1])
                i = escape + 2
                continue
            piece, i = _read_escape(source, escape + 1, is_bytes)
            if piece is not None:
                units.append(piece)
            continue
        if quote > i:
            units.append(source[i:quote])
        i = quote + len(closer)
        break
    if is_bytes:
        numbers = []
        for unit in units:
            if not isinstance(unit, str) or len(unit) > 1:
                for piece in unit:
                    point = ord(piece)
                    if point > 255:
                        raise 'SyntaxError: bytes can only contain ASCII literal characters'
                    numbers.append(point)
                continue
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
        try:
            named = unicodedata.lookup(source[i + 2:close])
        except KeyError:
            named = None
        if named is None or len(named) != 1:
            raise 'SyntaxError: unknown Unicode character name'
        return (named, close + 1)
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

    # Read the next parser token.
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

# Evaluate literal trees without evaluating arbitrary source code.
def literal_eval(node_or_string):
    if isinstance(node_or_string, str):
        node_or_string = parse(node_or_string.lstrip(' \t'), mode='eval')
    return _literal_node(node_or_string)

# --------------------------------------------------------------------------
# The source reader keeps token spans separate from expression spans so
# parentheses, decorators, and statement headers retain CPython's positions.
# Unsupported syntax raises an error instead of supplying a partial tree.

def _syntax(message):
    raise 'SyntaxError: ' + message

def _no_tree():
    raise _NO_TREE

# The native AST makers take the call's keyword mapping as it stands: a key
# that is no string is not refused at the call, but reaches the maker, which
# reports it with its own repr as CPython's C constructor does. This mark tells
# the runtime not to narrow such a key to a word before the maker sees it.
def _native_keywords(method):
    method._lumen_raw_keywords = True
    return method

class AST:
    _fields = ()
    _attributes = ()

    # Apply ASDL defaults while preserving the deprecated incomplete constructors.
    @_native_keywords
    def __init__(self, *args, **kwargs):
        cls = type(self)
        name = cls.__name__
        fields = self._fields
        if len(args) > len(fields):
            raise TypeError(name + ' constructor takes at most ' + str(len(fields)) + ' positional arguments')
        given = len(args)
        for index, value in enumerate(args):
            field = fields[index]
            if field in kwargs:
                # The reference names the keyword's own key, so a key that is
                # no string is reported by its repr rather than by the field.
                key = field
                for other in kwargs:
                    if other == field:
                        key = other
                        break
                raise TypeError(name + " got multiple values for argument " + repr(key))
            setattr(self, field, value)
        attributes = self._attributes
        for field, value in kwargs.items():
            if field in fields:
                given += 1
            elif field not in attributes:
                warnings.warn(name + ".__init__ got an unexpected keyword argument " + repr(field) + ". Support for arbitrary keyword arguments is deprecated and will be removed in Python 3.15.", DeprecationWarning, stacklevel=2)
            # A key that is no string still names a field the maker knows, but
            # no attribute can be written under it, so the complaint is the
            # reference's own about the kind of the attribute name.
            if not isinstance(field, str):
                raise TypeError("attribute name must be string, not '" + type(field).__name__ + "'")
            setattr(self, field, value)
        # Every field was supplied by position or keyword: no ASDL defaults
        # are needed, which is how the reader builds nearly every node.
        if given == len(fields):
            return
        types = getattr(cls, '_field_types', None)
        if types is None:
            return
        for field in fields[len(args):]:
            if field in kwargs:
                continue
            if field not in types:
                warnings.warn('Field ' + repr(field) + ' is missing from ' + name + '._field_types', DeprecationWarning, stacklevel=2)
                continue
            typ = types[field]
            if getattr(typ, '__origin__', None) is list:
                setattr(self, field, [])
            elif typ is expr_context:
                setattr(self, field, Load())
            elif not hasattr(cls, field):
                warnings.warn(name + ".__init__ missing 1 required positional argument: " + repr(field) + ". This will become an error in Python 3.15.", DeprecationWarning, stacklevel=2)

    # Replace only declared class fields and attributes, keeping shallow values.
    @_native_keywords
    def __replace__(self, **changes):
        cls = type(self)
        allowed = cls._fields + cls._attributes
        for name in changes:
            if name not in allowed:
                raise TypeError(cls.__name__ + ".__replace__ got an unexpected keyword argument " + repr(name) + '.')
        values = {}
        for name in allowed:
            if name in changes:
                values[name] = changes[name]
            elif hasattr(self, name):
                values[name] = getattr(self, name)
            elif name in cls._fields:
                raise TypeError(cls.__name__ + ".__replace__ missing 1 keyword argument: " + repr(name) + '.')
        return cls(**values)

    # Represent the actual fields, without source positions or private parser state.
    def __repr__(self):
        parts = []
        for field in self._fields:
            try:
                value = getattr(self, field)
            except AttributeError:
                continue
            parts.append(field + '=' + repr(value))
        return type(self).__name__ + '(' + ', '.join(parts) + ')'


class mod(AST):
    _fields = ()
    _attributes = ()

class Module(mod):
    _fields = ('body', 'type_ignores')
    _attributes = ()

class Interactive(mod):
    _fields = ('body',)
    _attributes = ()

class Expression(mod):
    _fields = ('body',)
    _attributes = ()

class FunctionType(mod):
    _fields = ('argtypes', 'returns')
    _attributes = ()

class stmt(AST):
    _fields = ()
    _attributes = ('lineno', 'col_offset', 'end_lineno', 'end_col_offset')
    end_lineno = None
    end_col_offset = None


class AsyncFunctionDef(stmt):
    _fields = ('name', 'args', 'body', 'decorator_list', 'returns', 'type_comment', 'type_params')
    _attributes = ('lineno', 'col_offset', 'end_lineno', 'end_col_offset')
    returns = None
    type_comment = None
    end_lineno = None
    end_col_offset = None

class ClassDef(stmt):
    _fields = ('name', 'bases', 'keywords', 'body', 'decorator_list', 'type_params')
    _attributes = ('lineno', 'col_offset', 'end_lineno', 'end_col_offset')
    end_lineno = None
    end_col_offset = None


class Delete(stmt):
    _fields = ('targets',)
    _attributes = ('lineno', 'col_offset', 'end_lineno', 'end_col_offset')
    end_lineno = None
    end_col_offset = None

class Assign(stmt):
    _fields = ('targets', 'value', 'type_comment')
    _attributes = ('lineno', 'col_offset', 'end_lineno', 'end_col_offset')
    type_comment = None
    end_lineno = None
    end_col_offset = None

class TypeAlias(stmt):
    _fields = ('name', 'type_params', 'value')
    _attributes = ('lineno', 'col_offset', 'end_lineno', 'end_col_offset')
    end_lineno = None
    end_col_offset = None

class AugAssign(stmt):
    _fields = ('target', 'op', 'value')
    _attributes = ('lineno', 'col_offset', 'end_lineno', 'end_col_offset')
    end_lineno = None
    end_col_offset = None

class AnnAssign(stmt):
    _fields = ('target', 'annotation', 'value', 'simple')
    _attributes = ('lineno', 'col_offset', 'end_lineno', 'end_col_offset')
    value = None
    end_lineno = None
    end_col_offset = None

class For(stmt):
    _fields = ('target', 'iter', 'body', 'orelse', 'type_comment')
    _attributes = ('lineno', 'col_offset', 'end_lineno', 'end_col_offset')
    type_comment = None
    end_lineno = None
    end_col_offset = None

class AsyncFor(stmt):
    _fields = ('target', 'iter', 'body', 'orelse', 'type_comment')
    _attributes = ('lineno', 'col_offset', 'end_lineno', 'end_col_offset')
    type_comment = None
    end_lineno = None
    end_col_offset = None

class While(stmt):
    _fields = ('test', 'body', 'orelse')
    _attributes = ('lineno', 'col_offset', 'end_lineno', 'end_col_offset')
    end_lineno = None
    end_col_offset = None


class With(stmt):
    _fields = ('items', 'body', 'type_comment')
    _attributes = ('lineno', 'col_offset', 'end_lineno', 'end_col_offset')
    type_comment = None
    end_lineno = None
    end_col_offset = None

class AsyncWith(stmt):
    _fields = ('items', 'body', 'type_comment')
    _attributes = ('lineno', 'col_offset', 'end_lineno', 'end_col_offset')
    type_comment = None
    end_lineno = None
    end_col_offset = None

class Match(stmt):
    _fields = ('subject', 'cases')
    _attributes = ('lineno', 'col_offset', 'end_lineno', 'end_col_offset')
    end_lineno = None
    end_col_offset = None

class Raise(stmt):
    _fields = ('exc', 'cause')
    _attributes = ('lineno', 'col_offset', 'end_lineno', 'end_col_offset')
    exc = None
    cause = None
    end_lineno = None
    end_col_offset = None


class TryStar(stmt):
    _fields = ('body', 'handlers', 'orelse', 'finalbody')
    _attributes = ('lineno', 'col_offset', 'end_lineno', 'end_col_offset')
    end_lineno = None
    end_col_offset = None


class Import(stmt):
    _fields = ('names',)
    _attributes = ('lineno', 'col_offset', 'end_lineno', 'end_col_offset')
    end_lineno = None
    end_col_offset = None

class ImportFrom(stmt):
    _fields = ('module', 'names', 'level')
    _attributes = ('lineno', 'col_offset', 'end_lineno', 'end_col_offset')
    module = None
    level = None
    end_lineno = None
    end_col_offset = None

class Global(stmt):
    _fields = ('names',)
    _attributes = ('lineno', 'col_offset', 'end_lineno', 'end_col_offset')
    end_lineno = None
    end_col_offset = None

class Nonlocal(stmt):
    _fields = ('names',)
    _attributes = ('lineno', 'col_offset', 'end_lineno', 'end_col_offset')
    end_lineno = None
    end_col_offset = None

class Expr(stmt):
    _fields = ('value',)
    _attributes = ('lineno', 'col_offset', 'end_lineno', 'end_col_offset')
    end_lineno = None
    end_col_offset = None


class Break(stmt):
    _fields = ()
    _attributes = ('lineno', 'col_offset', 'end_lineno', 'end_col_offset')
    end_lineno = None
    end_col_offset = None

class Continue(stmt):
    _fields = ()
    _attributes = ('lineno', 'col_offset', 'end_lineno', 'end_col_offset')
    end_lineno = None
    end_col_offset = None

class expr(AST):
    _fields = ()
    _attributes = ('lineno', 'col_offset', 'end_lineno', 'end_col_offset')
    end_lineno = None
    end_col_offset = None

class BoolOp(expr):
    _fields = ('op', 'values')
    _attributes = ('lineno', 'col_offset', 'end_lineno', 'end_col_offset')
    end_lineno = None
    end_col_offset = None

class NamedExpr(expr):
    _fields = ('target', 'value')
    _attributes = ('lineno', 'col_offset', 'end_lineno', 'end_col_offset')
    end_lineno = None
    end_col_offset = None

class BinOp(expr):
    _fields = ('left', 'op', 'right')
    _attributes = ('lineno', 'col_offset', 'end_lineno', 'end_col_offset')
    end_lineno = None
    end_col_offset = None

class UnaryOp(expr):
    _fields = ('op', 'operand')
    _attributes = ('lineno', 'col_offset', 'end_lineno', 'end_col_offset')
    end_lineno = None
    end_col_offset = None

class Lambda(expr):
    _fields = ('args', 'body')
    _attributes = ('lineno', 'col_offset', 'end_lineno', 'end_col_offset')
    end_lineno = None
    end_col_offset = None

class IfExp(expr):
    _fields = ('test', 'body', 'orelse')
    _attributes = ('lineno', 'col_offset', 'end_lineno', 'end_col_offset')
    end_lineno = None
    end_col_offset = None

class Dict(expr):
    _fields = ('keys', 'values')
    _attributes = ('lineno', 'col_offset', 'end_lineno', 'end_col_offset')
    end_lineno = None
    end_col_offset = None

class Set(expr):
    _fields = ('elts',)
    _attributes = ('lineno', 'col_offset', 'end_lineno', 'end_col_offset')
    end_lineno = None
    end_col_offset = None

class ListComp(expr):
    _fields = ('elt', 'generators')
    _attributes = ('lineno', 'col_offset', 'end_lineno', 'end_col_offset')
    end_lineno = None
    end_col_offset = None

class SetComp(expr):
    _fields = ('elt', 'generators')
    _attributes = ('lineno', 'col_offset', 'end_lineno', 'end_col_offset')
    end_lineno = None
    end_col_offset = None

class DictComp(expr):
    _fields = ('key', 'value', 'generators')
    _attributes = ('lineno', 'col_offset', 'end_lineno', 'end_col_offset')
    end_lineno = None
    end_col_offset = None

class GeneratorExp(expr):
    _fields = ('elt', 'generators')
    _attributes = ('lineno', 'col_offset', 'end_lineno', 'end_col_offset')
    end_lineno = None
    end_col_offset = None

class Await(expr):
    _fields = ('value',)
    _attributes = ('lineno', 'col_offset', 'end_lineno', 'end_col_offset')
    end_lineno = None
    end_col_offset = None

class Yield(expr):
    _fields = ('value',)
    _attributes = ('lineno', 'col_offset', 'end_lineno', 'end_col_offset')
    value = None
    end_lineno = None
    end_col_offset = None

class YieldFrom(expr):
    _fields = ('value',)
    _attributes = ('lineno', 'col_offset', 'end_lineno', 'end_col_offset')
    end_lineno = None
    end_col_offset = None

class Compare(expr):
    _fields = ('left', 'ops', 'comparators')
    _attributes = ('lineno', 'col_offset', 'end_lineno', 'end_col_offset')
    end_lineno = None
    end_col_offset = None

class Call(expr):
    _fields = ('func', 'args', 'keywords')
    _attributes = ('lineno', 'col_offset', 'end_lineno', 'end_col_offset')
    end_lineno = None
    end_col_offset = None

class FormattedValue(expr):
    _fields = ('value', 'conversion', 'format_spec')
    _attributes = ('lineno', 'col_offset', 'end_lineno', 'end_col_offset')
    format_spec = None
    end_lineno = None
    end_col_offset = None

class Interpolation(expr):
    _fields = ('value', 'str', 'conversion', 'format_spec')
    _attributes = ('lineno', 'col_offset', 'end_lineno', 'end_col_offset')
    format_spec = None
    end_lineno = None
    end_col_offset = None

class JoinedStr(expr):
    _fields = ('values',)
    _attributes = ('lineno', 'col_offset', 'end_lineno', 'end_col_offset')
    end_lineno = None
    end_col_offset = None

class TemplateStr(expr):
    _fields = ('values',)
    _attributes = ('lineno', 'col_offset', 'end_lineno', 'end_col_offset')
    end_lineno = None
    end_col_offset = None

class Constant(expr):
    _fields = ('value', 'kind')
    _attributes = ('lineno', 'col_offset', 'end_lineno', 'end_col_offset')
    kind = None
    end_lineno = None
    end_col_offset = None

class Attribute(expr):
    _fields = ('value', 'attr', 'ctx')
    _attributes = ('lineno', 'col_offset', 'end_lineno', 'end_col_offset')
    end_lineno = None
    end_col_offset = None

class Subscript(expr):
    _fields = ('value', 'slice', 'ctx')
    _attributes = ('lineno', 'col_offset', 'end_lineno', 'end_col_offset')
    end_lineno = None
    end_col_offset = None

class Starred(expr):
    _fields = ('value', 'ctx')
    _attributes = ('lineno', 'col_offset', 'end_lineno', 'end_col_offset')
    end_lineno = None
    end_col_offset = None

class Name(expr):
    _fields = ('id', 'ctx')
    _attributes = ('lineno', 'col_offset', 'end_lineno', 'end_col_offset')
    end_lineno = None
    end_col_offset = None

class List(expr):
    _fields = ('elts', 'ctx')
    _attributes = ('lineno', 'col_offset', 'end_lineno', 'end_col_offset')
    end_lineno = None
    end_col_offset = None

class Tuple(expr):
    _fields = ('elts', 'ctx')
    _attributes = ('lineno', 'col_offset', 'end_lineno', 'end_col_offset')
    end_lineno = None
    end_col_offset = None

class Slice(expr):
    _fields = ('lower', 'upper', 'step')
    _attributes = ('lineno', 'col_offset', 'end_lineno', 'end_col_offset')
    lower = None
    upper = None
    step = None
    end_lineno = None
    end_col_offset = None

class Pass(stmt):
    _fields = ()
    _attributes = ('lineno', 'col_offset', 'end_lineno', 'end_col_offset')
    end_lineno = None
    end_col_offset = None

class Return(stmt):
    _fields = ('value',)
    _attributes = ('lineno', 'col_offset', 'end_lineno', 'end_col_offset')
    value = None
    end_lineno = None
    end_col_offset = None

class Assert(stmt):
    _fields = ('test', 'msg')
    _attributes = ('lineno', 'col_offset', 'end_lineno', 'end_col_offset')
    msg = None
    end_lineno = None
    end_col_offset = None

class If(stmt):
    _fields = ('test', 'body', 'orelse')
    _attributes = ('lineno', 'col_offset', 'end_lineno', 'end_col_offset')
    end_lineno = None
    end_col_offset = None

class Try(stmt):
    _fields = ('body', 'handlers', 'orelse', 'finalbody')
    _attributes = ('lineno', 'col_offset', 'end_lineno', 'end_col_offset')
    end_lineno = None
    end_col_offset = None


class FunctionDef(stmt):
    _fields = ('name', 'args', 'body', 'decorator_list', 'returns',
               'type_comment', 'type_params')
    _attributes = ('lineno', 'col_offset', 'end_lineno', 'end_col_offset')
    returns = None
    type_comment = None
    end_lineno = None
    end_col_offset = None



class expr_context(AST):
    _fields = ()
    _attributes = ()

class Load(expr_context):
    _fields = ()
    _attributes = ()

class Store(expr_context):
    _fields = ()
    _attributes = ()

class Del(expr_context):
    _fields = ()
    _attributes = ()

class boolop(AST):
    _fields = ()
    _attributes = ()

class And(boolop):
    _fields = ()
    _attributes = ()

class Or(boolop):
    _fields = ()
    _attributes = ()

class operator(AST):
    _fields = ()
    _attributes = ()

class Add(operator):
    _fields = ()
    _attributes = ()

class Sub(operator):
    _fields = ()
    _attributes = ()

class Mult(operator):
    _fields = ()
    _attributes = ()

class MatMult(operator):
    _fields = ()
    _attributes = ()

class Div(operator):
    _fields = ()
    _attributes = ()

class Mod(operator):
    _fields = ()
    _attributes = ()

class Pow(operator):
    _fields = ()
    _attributes = ()

class LShift(operator):
    _fields = ()
    _attributes = ()

class RShift(operator):
    _fields = ()
    _attributes = ()

class BitOr(operator):
    _fields = ()
    _attributes = ()

class BitXor(operator):
    _fields = ()
    _attributes = ()

class BitAnd(operator):
    _fields = ()
    _attributes = ()

class FloorDiv(operator):
    _fields = ()
    _attributes = ()

class unaryop(AST):
    _fields = ()
    _attributes = ()

class Invert(unaryop):
    _fields = ()
    _attributes = ()

class Not(unaryop):
    _fields = ()
    _attributes = ()

class UAdd(unaryop):
    _fields = ()
    _attributes = ()

class USub(unaryop):
    _fields = ()
    _attributes = ()

class cmpop(AST):
    _fields = ()
    _attributes = ()

class Eq(cmpop):
    _fields = ()
    _attributes = ()

class NotEq(cmpop):
    _fields = ()
    _attributes = ()

class Lt(cmpop):
    _fields = ()
    _attributes = ()

class LtE(cmpop):
    _fields = ()
    _attributes = ()

class Gt(cmpop):
    _fields = ()
    _attributes = ()

class GtE(cmpop):
    _fields = ()
    _attributes = ()

class Is(cmpop):
    _fields = ()
    _attributes = ()

class IsNot(cmpop):
    _fields = ()
    _attributes = ()

class In(cmpop):
    _fields = ()
    _attributes = ()

class NotIn(cmpop):
    _fields = ()
    _attributes = ()

class comprehension(AST):
    _fields = ('target', 'iter', 'ifs', 'is_async')
    _attributes = ()

class excepthandler(AST):
    _fields = ()
    _attributes = ('lineno', 'col_offset', 'end_lineno', 'end_col_offset')
    end_lineno = None
    end_col_offset = None

class ExceptHandler(excepthandler):
    _fields = ('type', 'name', 'body')
    _attributes = ('lineno', 'col_offset', 'end_lineno', 'end_col_offset')
    type = None
    name = None
    end_lineno = None
    end_col_offset = None

class arguments(AST):
    _fields = ('posonlyargs', 'args', 'vararg', 'kwonlyargs', 'kw_defaults', 'kwarg', 'defaults')
    _attributes = ()
    vararg = None
    kwarg = None

class arg(AST):
    _fields = ('arg', 'annotation', 'type_comment')
    _attributes = ('lineno', 'col_offset', 'end_lineno', 'end_col_offset')
    annotation = None
    type_comment = None
    end_lineno = None
    end_col_offset = None

class keyword(AST):
    _fields = ('arg', 'value')
    _attributes = ('lineno', 'col_offset', 'end_lineno', 'end_col_offset')
    arg = None
    end_lineno = None
    end_col_offset = None

class alias(AST):
    _fields = ('name', 'asname')
    _attributes = ('lineno', 'col_offset', 'end_lineno', 'end_col_offset')
    asname = None
    end_lineno = None
    end_col_offset = None

class withitem(AST):
    _fields = ('context_expr', 'optional_vars')
    _attributes = ()
    optional_vars = None

class match_case(AST):
    _fields = ('pattern', 'guard', 'body')
    _attributes = ()
    guard = None

class pattern(AST):
    _fields = ()
    _attributes = ('lineno', 'col_offset', 'end_lineno', 'end_col_offset')

class MatchValue(pattern):
    _fields = ('value',)
    _attributes = ('lineno', 'col_offset', 'end_lineno', 'end_col_offset')

class MatchSingleton(pattern):
    _fields = ('value',)
    _attributes = ('lineno', 'col_offset', 'end_lineno', 'end_col_offset')

class MatchSequence(pattern):
    _fields = ('patterns',)
    _attributes = ('lineno', 'col_offset', 'end_lineno', 'end_col_offset')

class MatchMapping(pattern):
    _fields = ('keys', 'patterns', 'rest')
    _attributes = ('lineno', 'col_offset', 'end_lineno', 'end_col_offset')
    rest = None

class MatchClass(pattern):
    _fields = ('cls', 'patterns', 'kwd_attrs', 'kwd_patterns')
    _attributes = ('lineno', 'col_offset', 'end_lineno', 'end_col_offset')

class MatchStar(pattern):
    _fields = ('name',)
    _attributes = ('lineno', 'col_offset', 'end_lineno', 'end_col_offset')
    name = None

class MatchAs(pattern):
    _fields = ('pattern', 'name')
    _attributes = ('lineno', 'col_offset', 'end_lineno', 'end_col_offset')
    pattern = None
    name = None

class MatchOr(pattern):
    _fields = ('patterns',)
    _attributes = ('lineno', 'col_offset', 'end_lineno', 'end_col_offset')

class type_ignore(AST):
    _fields = ()
    _attributes = ()

class TypeIgnore(type_ignore):
    _fields = ('lineno', 'tag')
    _attributes = ()

class type_param(AST):
    _fields = ()
    _attributes = ('lineno', 'col_offset', 'end_lineno', 'end_col_offset')

class TypeVar(type_param):
    _fields = ('name', 'bound', 'default_value')
    _attributes = ('lineno', 'col_offset', 'end_lineno', 'end_col_offset')
    bound = None
    default_value = None

class ParamSpec(type_param):
    _fields = ('name', 'default_value')
    _attributes = ('lineno', 'col_offset', 'end_lineno', 'end_col_offset')
    default_value = None

class TypeVarTuple(type_param):
    _fields = ('name', 'default_value')
    _attributes = ('lineno', 'col_offset', 'end_lineno', 'end_col_offset')
    default_value = None
mod.__annotations__ = {}
Module._field_types = {'body': list[stmt], 'type_ignores': list[type_ignore]}
Module.__annotations__ = Module._field_types
Interactive._field_types = {'body': list[stmt]}
Interactive.__annotations__ = Interactive._field_types
Expression._field_types = {'body': expr}
Expression.__annotations__ = Expression._field_types
FunctionType._field_types = {'argtypes': list[expr], 'returns': expr}
FunctionType.__annotations__ = FunctionType._field_types
stmt.__annotations__ = {}
FunctionDef._field_types = {'name': str, 'args': arguments, 'body': list[stmt], 'decorator_list': list[expr], 'returns': expr | None, 'type_comment': str | None, 'type_params': list[type_param]}
FunctionDef.__annotations__ = FunctionDef._field_types
AsyncFunctionDef._field_types = {'name': str, 'args': arguments, 'body': list[stmt], 'decorator_list': list[expr], 'returns': expr | None, 'type_comment': str | None, 'type_params': list[type_param]}
AsyncFunctionDef.__annotations__ = AsyncFunctionDef._field_types
ClassDef._field_types = {'name': str, 'bases': list[expr], 'keywords': list[keyword], 'body': list[stmt], 'decorator_list': list[expr], 'type_params': list[type_param]}
ClassDef.__annotations__ = ClassDef._field_types
Return._field_types = {'value': expr | None}
Return.__annotations__ = Return._field_types
Delete._field_types = {'targets': list[expr]}
Delete.__annotations__ = Delete._field_types
Assign._field_types = {'targets': list[expr], 'value': expr, 'type_comment': str | None}
Assign.__annotations__ = Assign._field_types
TypeAlias._field_types = {'name': expr, 'type_params': list[type_param], 'value': expr}
TypeAlias.__annotations__ = TypeAlias._field_types
AugAssign._field_types = {'target': expr, 'op': operator, 'value': expr}
AugAssign.__annotations__ = AugAssign._field_types
AnnAssign._field_types = {'target': expr, 'annotation': expr, 'value': expr | None, 'simple': int}
AnnAssign.__annotations__ = AnnAssign._field_types
For._field_types = {'target': expr, 'iter': expr, 'body': list[stmt], 'orelse': list[stmt], 'type_comment': str | None}
For.__annotations__ = For._field_types
AsyncFor._field_types = {'target': expr, 'iter': expr, 'body': list[stmt], 'orelse': list[stmt], 'type_comment': str | None}
AsyncFor.__annotations__ = AsyncFor._field_types
While._field_types = {'test': expr, 'body': list[stmt], 'orelse': list[stmt]}
While.__annotations__ = While._field_types
If._field_types = {'test': expr, 'body': list[stmt], 'orelse': list[stmt]}
If.__annotations__ = If._field_types
With._field_types = {'items': list[withitem], 'body': list[stmt], 'type_comment': str | None}
With.__annotations__ = With._field_types
AsyncWith._field_types = {'items': list[withitem], 'body': list[stmt], 'type_comment': str | None}
AsyncWith.__annotations__ = AsyncWith._field_types
Match._field_types = {'subject': expr, 'cases': list[match_case]}
Match.__annotations__ = Match._field_types
Raise._field_types = {'exc': expr | None, 'cause': expr | None}
Raise.__annotations__ = Raise._field_types
Try._field_types = {'body': list[stmt], 'handlers': list[excepthandler], 'orelse': list[stmt], 'finalbody': list[stmt]}
Try.__annotations__ = Try._field_types
TryStar._field_types = {'body': list[stmt], 'handlers': list[excepthandler], 'orelse': list[stmt], 'finalbody': list[stmt]}
TryStar.__annotations__ = TryStar._field_types
Assert._field_types = {'test': expr, 'msg': expr | None}
Assert.__annotations__ = Assert._field_types
Import._field_types = {'names': list[alias]}
Import.__annotations__ = Import._field_types
ImportFrom._field_types = {'module': str | None, 'names': list[alias], 'level': int | None}
ImportFrom.__annotations__ = ImportFrom._field_types
Global._field_types = {'names': list[str]}
Global.__annotations__ = Global._field_types
Nonlocal._field_types = {'names': list[str]}
Nonlocal.__annotations__ = Nonlocal._field_types
Expr._field_types = {'value': expr}
Expr.__annotations__ = Expr._field_types
Pass._field_types = {}
Pass.__annotations__ = Pass._field_types
Break._field_types = {}
Break.__annotations__ = Break._field_types
Continue._field_types = {}
Continue.__annotations__ = Continue._field_types
expr.__annotations__ = {}
BoolOp._field_types = {'op': boolop, 'values': list[expr]}
BoolOp.__annotations__ = BoolOp._field_types
NamedExpr._field_types = {'target': expr, 'value': expr}
NamedExpr.__annotations__ = NamedExpr._field_types
BinOp._field_types = {'left': expr, 'op': operator, 'right': expr}
BinOp.__annotations__ = BinOp._field_types
UnaryOp._field_types = {'op': unaryop, 'operand': expr}
UnaryOp.__annotations__ = UnaryOp._field_types
Lambda._field_types = {'args': arguments, 'body': expr}
Lambda.__annotations__ = Lambda._field_types
IfExp._field_types = {'test': expr, 'body': expr, 'orelse': expr}
IfExp.__annotations__ = IfExp._field_types
Dict._field_types = {'keys': list[expr], 'values': list[expr]}
Dict.__annotations__ = Dict._field_types
Set._field_types = {'elts': list[expr]}
Set.__annotations__ = Set._field_types
ListComp._field_types = {'elt': expr, 'generators': list[comprehension]}
ListComp.__annotations__ = ListComp._field_types
SetComp._field_types = {'elt': expr, 'generators': list[comprehension]}
SetComp.__annotations__ = SetComp._field_types
DictComp._field_types = {'key': expr, 'value': expr, 'generators': list[comprehension]}
DictComp.__annotations__ = DictComp._field_types
GeneratorExp._field_types = {'elt': expr, 'generators': list[comprehension]}
GeneratorExp.__annotations__ = GeneratorExp._field_types
Await._field_types = {'value': expr}
Await.__annotations__ = Await._field_types
Yield._field_types = {'value': expr | None}
Yield.__annotations__ = Yield._field_types
YieldFrom._field_types = {'value': expr}
YieldFrom.__annotations__ = YieldFrom._field_types
Compare._field_types = {'left': expr, 'ops': list[cmpop], 'comparators': list[expr]}
Compare.__annotations__ = Compare._field_types
Call._field_types = {'func': expr, 'args': list[expr], 'keywords': list[keyword]}
Call.__annotations__ = Call._field_types
FormattedValue._field_types = {'value': expr, 'conversion': int, 'format_spec': expr | None}
FormattedValue.__annotations__ = FormattedValue._field_types
Interpolation._field_types = {'value': expr, 'str': object, 'conversion': int, 'format_spec': expr | None}
Interpolation.__annotations__ = Interpolation._field_types
JoinedStr._field_types = {'values': list[expr]}
JoinedStr.__annotations__ = JoinedStr._field_types
TemplateStr._field_types = {'values': list[expr]}
TemplateStr.__annotations__ = TemplateStr._field_types
Constant._field_types = {'value': object, 'kind': str | None}
Constant.__annotations__ = Constant._field_types
Attribute._field_types = {'value': expr, 'attr': str, 'ctx': expr_context}
Attribute.__annotations__ = Attribute._field_types
Subscript._field_types = {'value': expr, 'slice': expr, 'ctx': expr_context}
Subscript.__annotations__ = Subscript._field_types
Starred._field_types = {'value': expr, 'ctx': expr_context}
Starred.__annotations__ = Starred._field_types
Name._field_types = {'id': str, 'ctx': expr_context}
Name.__annotations__ = Name._field_types
List._field_types = {'elts': list[expr], 'ctx': expr_context}
List.__annotations__ = List._field_types
Tuple._field_types = {'elts': list[expr], 'ctx': expr_context}
Tuple.__annotations__ = Tuple._field_types
Slice._field_types = {'lower': expr | None, 'upper': expr | None, 'step': expr | None}
Slice.__annotations__ = Slice._field_types
expr_context.__annotations__ = {}
Load._field_types = {}
Load.__annotations__ = Load._field_types
Store._field_types = {}
Store.__annotations__ = Store._field_types
Del._field_types = {}
Del.__annotations__ = Del._field_types
boolop.__annotations__ = {}
And._field_types = {}
And.__annotations__ = And._field_types
Or._field_types = {}
Or.__annotations__ = Or._field_types
operator.__annotations__ = {}
Add._field_types = {}
Add.__annotations__ = Add._field_types
Sub._field_types = {}
Sub.__annotations__ = Sub._field_types
Mult._field_types = {}
Mult.__annotations__ = Mult._field_types
MatMult._field_types = {}
MatMult.__annotations__ = MatMult._field_types
Div._field_types = {}
Div.__annotations__ = Div._field_types
Mod._field_types = {}
Mod.__annotations__ = Mod._field_types
Pow._field_types = {}
Pow.__annotations__ = Pow._field_types
LShift._field_types = {}
LShift.__annotations__ = LShift._field_types
RShift._field_types = {}
RShift.__annotations__ = RShift._field_types
BitOr._field_types = {}
BitOr.__annotations__ = BitOr._field_types
BitXor._field_types = {}
BitXor.__annotations__ = BitXor._field_types
BitAnd._field_types = {}
BitAnd.__annotations__ = BitAnd._field_types
FloorDiv._field_types = {}
FloorDiv.__annotations__ = FloorDiv._field_types
unaryop.__annotations__ = {}
Invert._field_types = {}
Invert.__annotations__ = Invert._field_types
Not._field_types = {}
Not.__annotations__ = Not._field_types
UAdd._field_types = {}
UAdd.__annotations__ = UAdd._field_types
USub._field_types = {}
USub.__annotations__ = USub._field_types
cmpop.__annotations__ = {}
Eq._field_types = {}
Eq.__annotations__ = Eq._field_types
NotEq._field_types = {}
NotEq.__annotations__ = NotEq._field_types
Lt._field_types = {}
Lt.__annotations__ = Lt._field_types
LtE._field_types = {}
LtE.__annotations__ = LtE._field_types
Gt._field_types = {}
Gt.__annotations__ = Gt._field_types
GtE._field_types = {}
GtE.__annotations__ = GtE._field_types
Is._field_types = {}
Is.__annotations__ = Is._field_types
IsNot._field_types = {}
IsNot.__annotations__ = IsNot._field_types
In._field_types = {}
In.__annotations__ = In._field_types
NotIn._field_types = {}
NotIn.__annotations__ = NotIn._field_types
comprehension._field_types = {'target': expr, 'iter': expr, 'ifs': list[expr], 'is_async': int}
comprehension.__annotations__ = comprehension._field_types
excepthandler.__annotations__ = {}
ExceptHandler._field_types = {'type': expr | None, 'name': str | None, 'body': list[stmt]}
ExceptHandler.__annotations__ = ExceptHandler._field_types
arguments._field_types = {'posonlyargs': list[arg], 'args': list[arg], 'vararg': arg | None, 'kwonlyargs': list[arg], 'kw_defaults': list[expr], 'kwarg': arg | None, 'defaults': list[expr]}
arguments.__annotations__ = arguments._field_types
arg._field_types = {'arg': str, 'annotation': expr | None, 'type_comment': str | None}
arg.__annotations__ = arg._field_types
keyword._field_types = {'arg': str | None, 'value': expr}
keyword.__annotations__ = keyword._field_types
alias._field_types = {'name': str, 'asname': str | None}
alias.__annotations__ = alias._field_types
withitem._field_types = {'context_expr': expr, 'optional_vars': expr | None}
withitem.__annotations__ = withitem._field_types
match_case._field_types = {'pattern': pattern, 'guard': expr | None, 'body': list[stmt]}
match_case.__annotations__ = match_case._field_types
pattern.__annotations__ = {}
MatchValue._field_types = {'value': expr}
MatchValue.__annotations__ = MatchValue._field_types
MatchSingleton._field_types = {'value': object}
MatchSingleton.__annotations__ = MatchSingleton._field_types
MatchSequence._field_types = {'patterns': list[pattern]}
MatchSequence.__annotations__ = MatchSequence._field_types
MatchMapping._field_types = {'keys': list[expr], 'patterns': list[pattern], 'rest': str | None}
MatchMapping.__annotations__ = MatchMapping._field_types
MatchClass._field_types = {'cls': expr, 'patterns': list[pattern], 'kwd_attrs': list[str], 'kwd_patterns': list[pattern]}
MatchClass.__annotations__ = MatchClass._field_types
MatchStar._field_types = {'name': str | None}
MatchStar.__annotations__ = MatchStar._field_types
MatchAs._field_types = {'pattern': pattern | None, 'name': str | None}
MatchAs.__annotations__ = MatchAs._field_types
MatchOr._field_types = {'patterns': list[pattern]}
MatchOr.__annotations__ = MatchOr._field_types
type_ignore.__annotations__ = {}
TypeIgnore._field_types = {'lineno': int, 'tag': str}
TypeIgnore.__annotations__ = TypeIgnore._field_types
type_param.__annotations__ = {}
TypeVar._field_types = {'name': str, 'bound': expr | None, 'default_value': expr | None}
TypeVar.__annotations__ = TypeVar._field_types
ParamSpec._field_types = {'name': str, 'default_value': expr | None}
ParamSpec.__annotations__ = ParamSpec._field_types
TypeVarTuple._field_types = {'name': str, 'default_value': expr | None}
TypeVarTuple.__annotations__ = TypeVarTuple._field_types


mod.__doc__ = 'mod = Module(stmt* body, type_ignore* type_ignores)\n    | Interactive(stmt* body)\n    | Expression(expr body)\n    | FunctionType(expr* argtypes, expr returns)'
mod.__match_args__ = ()
Module.__doc__ = 'Module(stmt* body, type_ignore* type_ignores)'
Module.__match_args__ = ('body', 'type_ignores')
Interactive.__doc__ = 'Interactive(stmt* body)'
Interactive.__match_args__ = ('body',)
Expression.__doc__ = 'Expression(expr body)'
Expression.__match_args__ = ('body',)
FunctionType.__doc__ = 'FunctionType(expr* argtypes, expr returns)'
FunctionType.__match_args__ = ('argtypes', 'returns')
stmt.__doc__ = 'stmt = FunctionDef(identifier name, arguments args, stmt* body, expr* decorator_list, expr? returns, string? type_comment, type_param* type_params)\n     | AsyncFunctionDef(identifier name, arguments args, stmt* body, expr* decorator_list, expr? returns, string? type_comment, type_param* type_params)\n     | ClassDef(identifier name, expr* bases, keyword* keywords, stmt* body, expr* decorator_list, type_param* type_params)\n     | Return(expr? value)\n     | Delete(expr* targets)\n     | Assign(expr* targets, expr value, string? type_comment)\n     | TypeAlias(expr name, type_param* type_params, expr value)\n     | AugAssign(expr target, operator op, expr value)\n     | AnnAssign(expr target, expr annotation, expr? value, int simple)\n     | For(expr target, expr iter, stmt* body, stmt* orelse, string? type_comment)\n     | AsyncFor(expr target, expr iter, stmt* body, stmt* orelse, string? type_comment)\n     | While(expr test, stmt* body, stmt* orelse)\n     | If(expr test, stmt* body, stmt* orelse)\n     | With(withitem* items, stmt* body, string? type_comment)\n     | AsyncWith(withitem* items, stmt* body, string? type_comment)\n     | Match(expr subject, match_case* cases)\n     | Raise(expr? exc, expr? cause)\n     | Try(stmt* body, excepthandler* handlers, stmt* orelse, stmt* finalbody)\n     | TryStar(stmt* body, excepthandler* handlers, stmt* orelse, stmt* finalbody)\n     | Assert(expr test, expr? msg)\n     | Import(alias* names)\n     | ImportFrom(identifier? module, alias* names, int? level)\n     | Global(identifier* names)\n     | Nonlocal(identifier* names)\n     | Expr(expr value)\n     | Pass\n     | Break\n     | Continue'
stmt.__match_args__ = ()
FunctionDef.__doc__ = 'FunctionDef(identifier name, arguments args, stmt* body, expr* decorator_list, expr? returns, string? type_comment, type_param* type_params)'
FunctionDef.__match_args__ = ('name', 'args', 'body', 'decorator_list', 'returns', 'type_comment', 'type_params')
AsyncFunctionDef.__doc__ = 'AsyncFunctionDef(identifier name, arguments args, stmt* body, expr* decorator_list, expr? returns, string? type_comment, type_param* type_params)'
AsyncFunctionDef.__match_args__ = ('name', 'args', 'body', 'decorator_list', 'returns', 'type_comment', 'type_params')
ClassDef.__doc__ = 'ClassDef(identifier name, expr* bases, keyword* keywords, stmt* body, expr* decorator_list, type_param* type_params)'
ClassDef.__match_args__ = ('name', 'bases', 'keywords', 'body', 'decorator_list', 'type_params')
Return.__doc__ = 'Return(expr? value)'
Return.__match_args__ = ('value',)
Delete.__doc__ = 'Delete(expr* targets)'
Delete.__match_args__ = ('targets',)
Assign.__doc__ = 'Assign(expr* targets, expr value, string? type_comment)'
Assign.__match_args__ = ('targets', 'value', 'type_comment')
TypeAlias.__doc__ = 'TypeAlias(expr name, type_param* type_params, expr value)'
TypeAlias.__match_args__ = ('name', 'type_params', 'value')
AugAssign.__doc__ = 'AugAssign(expr target, operator op, expr value)'
AugAssign.__match_args__ = ('target', 'op', 'value')
AnnAssign.__doc__ = 'AnnAssign(expr target, expr annotation, expr? value, int simple)'
AnnAssign.__match_args__ = ('target', 'annotation', 'value', 'simple')
For.__doc__ = 'For(expr target, expr iter, stmt* body, stmt* orelse, string? type_comment)'
For.__match_args__ = ('target', 'iter', 'body', 'orelse', 'type_comment')
AsyncFor.__doc__ = 'AsyncFor(expr target, expr iter, stmt* body, stmt* orelse, string? type_comment)'
AsyncFor.__match_args__ = ('target', 'iter', 'body', 'orelse', 'type_comment')
While.__doc__ = 'While(expr test, stmt* body, stmt* orelse)'
While.__match_args__ = ('test', 'body', 'orelse')
If.__doc__ = 'If(expr test, stmt* body, stmt* orelse)'
If.__match_args__ = ('test', 'body', 'orelse')
With.__doc__ = 'With(withitem* items, stmt* body, string? type_comment)'
With.__match_args__ = ('items', 'body', 'type_comment')
AsyncWith.__doc__ = 'AsyncWith(withitem* items, stmt* body, string? type_comment)'
AsyncWith.__match_args__ = ('items', 'body', 'type_comment')
Match.__doc__ = 'Match(expr subject, match_case* cases)'
Match.__match_args__ = ('subject', 'cases')
Raise.__doc__ = 'Raise(expr? exc, expr? cause)'
Raise.__match_args__ = ('exc', 'cause')
Try.__doc__ = 'Try(stmt* body, excepthandler* handlers, stmt* orelse, stmt* finalbody)'
Try.__match_args__ = ('body', 'handlers', 'orelse', 'finalbody')
TryStar.__doc__ = 'TryStar(stmt* body, excepthandler* handlers, stmt* orelse, stmt* finalbody)'
TryStar.__match_args__ = ('body', 'handlers', 'orelse', 'finalbody')
Assert.__doc__ = 'Assert(expr test, expr? msg)'
Assert.__match_args__ = ('test', 'msg')
Import.__doc__ = 'Import(alias* names)'
Import.__match_args__ = ('names',)
ImportFrom.__doc__ = 'ImportFrom(identifier? module, alias* names, int? level)'
ImportFrom.__match_args__ = ('module', 'names', 'level')
Global.__doc__ = 'Global(identifier* names)'
Global.__match_args__ = ('names',)
Nonlocal.__doc__ = 'Nonlocal(identifier* names)'
Nonlocal.__match_args__ = ('names',)
Expr.__doc__ = 'Expr(expr value)'
Expr.__match_args__ = ('value',)
Pass.__doc__ = 'Pass'
Pass.__match_args__ = ()
Break.__doc__ = 'Break'
Break.__match_args__ = ()
Continue.__doc__ = 'Continue'
Continue.__match_args__ = ()
expr.__doc__ = 'expr = BoolOp(boolop op, expr* values)\n     | NamedExpr(expr target, expr value)\n     | BinOp(expr left, operator op, expr right)\n     | UnaryOp(unaryop op, expr operand)\n     | Lambda(arguments args, expr body)\n     | IfExp(expr test, expr body, expr orelse)\n     | Dict(expr?* keys, expr* values)\n     | Set(expr* elts)\n     | ListComp(expr elt, comprehension* generators)\n     | SetComp(expr elt, comprehension* generators)\n     | DictComp(expr key, expr value, comprehension* generators)\n     | GeneratorExp(expr elt, comprehension* generators)\n     | Await(expr value)\n     | Yield(expr? value)\n     | YieldFrom(expr value)\n     | Compare(expr left, cmpop* ops, expr* comparators)\n     | Call(expr func, expr* args, keyword* keywords)\n     | FormattedValue(expr value, int conversion, expr? format_spec)\n     | Interpolation(expr value, constant str, int conversion, expr? format_spec)\n     | JoinedStr(expr* values)\n     | TemplateStr(expr* values)\n     | Constant(constant value, string? kind)\n     | Attribute(expr value, identifier attr, expr_context ctx)\n     | Subscript(expr value, expr slice, expr_context ctx)\n     | Starred(expr value, expr_context ctx)\n     | Name(identifier id, expr_context ctx)\n     | List(expr* elts, expr_context ctx)\n     | Tuple(expr* elts, expr_context ctx)\n     | Slice(expr? lower, expr? upper, expr? step)'
expr.__match_args__ = ()
BoolOp.__doc__ = 'BoolOp(boolop op, expr* values)'
BoolOp.__match_args__ = ('op', 'values')
NamedExpr.__doc__ = 'NamedExpr(expr target, expr value)'
NamedExpr.__match_args__ = ('target', 'value')
BinOp.__doc__ = 'BinOp(expr left, operator op, expr right)'
BinOp.__match_args__ = ('left', 'op', 'right')
UnaryOp.__doc__ = 'UnaryOp(unaryop op, expr operand)'
UnaryOp.__match_args__ = ('op', 'operand')
Lambda.__doc__ = 'Lambda(arguments args, expr body)'
Lambda.__match_args__ = ('args', 'body')
IfExp.__doc__ = 'IfExp(expr test, expr body, expr orelse)'
IfExp.__match_args__ = ('test', 'body', 'orelse')
Dict.__doc__ = 'Dict(expr?* keys, expr* values)'
Dict.__match_args__ = ('keys', 'values')
Set.__doc__ = 'Set(expr* elts)'
Set.__match_args__ = ('elts',)
ListComp.__doc__ = 'ListComp(expr elt, comprehension* generators)'
ListComp.__match_args__ = ('elt', 'generators')
SetComp.__doc__ = 'SetComp(expr elt, comprehension* generators)'
SetComp.__match_args__ = ('elt', 'generators')
DictComp.__doc__ = 'DictComp(expr key, expr value, comprehension* generators)'
DictComp.__match_args__ = ('key', 'value', 'generators')
GeneratorExp.__doc__ = 'GeneratorExp(expr elt, comprehension* generators)'
GeneratorExp.__match_args__ = ('elt', 'generators')
Await.__doc__ = 'Await(expr value)'
Await.__match_args__ = ('value',)
Yield.__doc__ = 'Yield(expr? value)'
Yield.__match_args__ = ('value',)
YieldFrom.__doc__ = 'YieldFrom(expr value)'
YieldFrom.__match_args__ = ('value',)
Compare.__doc__ = 'Compare(expr left, cmpop* ops, expr* comparators)'
Compare.__match_args__ = ('left', 'ops', 'comparators')
Call.__doc__ = 'Call(expr func, expr* args, keyword* keywords)'
Call.__match_args__ = ('func', 'args', 'keywords')
FormattedValue.__doc__ = 'FormattedValue(expr value, int conversion, expr? format_spec)'
FormattedValue.__match_args__ = ('value', 'conversion', 'format_spec')
Interpolation.__doc__ = 'Interpolation(expr value, constant str, int conversion, expr? format_spec)'
Interpolation.__match_args__ = ('value', 'str', 'conversion', 'format_spec')
JoinedStr.__doc__ = 'JoinedStr(expr* values)'
JoinedStr.__match_args__ = ('values',)
TemplateStr.__doc__ = 'TemplateStr(expr* values)'
TemplateStr.__match_args__ = ('values',)
Constant.__doc__ = 'Constant(constant value, string? kind)'
Constant.__match_args__ = ('value', 'kind')
Attribute.__doc__ = 'Attribute(expr value, identifier attr, expr_context ctx)'
Attribute.__match_args__ = ('value', 'attr', 'ctx')
Subscript.__doc__ = 'Subscript(expr value, expr slice, expr_context ctx)'
Subscript.__match_args__ = ('value', 'slice', 'ctx')
Starred.__doc__ = 'Starred(expr value, expr_context ctx)'
Starred.__match_args__ = ('value', 'ctx')
Name.__doc__ = 'Name(identifier id, expr_context ctx)'
Name.__match_args__ = ('id', 'ctx')
List.__doc__ = 'List(expr* elts, expr_context ctx)'
List.__match_args__ = ('elts', 'ctx')
Tuple.__doc__ = 'Tuple(expr* elts, expr_context ctx)'
Tuple.__match_args__ = ('elts', 'ctx')
Slice.__doc__ = 'Slice(expr? lower, expr? upper, expr? step)'
Slice.__match_args__ = ('lower', 'upper', 'step')
expr_context.__doc__ = 'expr_context = Load | Store | Del'
expr_context.__match_args__ = ()
Load.__doc__ = 'Load'
Load.__match_args__ = ()
Store.__doc__ = 'Store'
Store.__match_args__ = ()
Del.__doc__ = 'Del'
Del.__match_args__ = ()
boolop.__doc__ = 'boolop = And | Or'
boolop.__match_args__ = ()
And.__doc__ = 'And'
And.__match_args__ = ()
Or.__doc__ = 'Or'
Or.__match_args__ = ()
operator.__doc__ = 'operator = Add | Sub | Mult | MatMult | Div | Mod | Pow | LShift | RShift | BitOr | BitXor | BitAnd | FloorDiv'
operator.__match_args__ = ()
Add.__doc__ = 'Add'
Add.__match_args__ = ()
Sub.__doc__ = 'Sub'
Sub.__match_args__ = ()
Mult.__doc__ = 'Mult'
Mult.__match_args__ = ()
MatMult.__doc__ = 'MatMult'
MatMult.__match_args__ = ()
Div.__doc__ = 'Div'
Div.__match_args__ = ()
Mod.__doc__ = 'Mod'
Mod.__match_args__ = ()
Pow.__doc__ = 'Pow'
Pow.__match_args__ = ()
LShift.__doc__ = 'LShift'
LShift.__match_args__ = ()
RShift.__doc__ = 'RShift'
RShift.__match_args__ = ()
BitOr.__doc__ = 'BitOr'
BitOr.__match_args__ = ()
BitXor.__doc__ = 'BitXor'
BitXor.__match_args__ = ()
BitAnd.__doc__ = 'BitAnd'
BitAnd.__match_args__ = ()
FloorDiv.__doc__ = 'FloorDiv'
FloorDiv.__match_args__ = ()
unaryop.__doc__ = 'unaryop = Invert | Not | UAdd | USub'
unaryop.__match_args__ = ()
Invert.__doc__ = 'Invert'
Invert.__match_args__ = ()
Not.__doc__ = 'Not'
Not.__match_args__ = ()
UAdd.__doc__ = 'UAdd'
UAdd.__match_args__ = ()
USub.__doc__ = 'USub'
USub.__match_args__ = ()
cmpop.__doc__ = 'cmpop = Eq | NotEq | Lt | LtE | Gt | GtE | Is | IsNot | In | NotIn'
cmpop.__match_args__ = ()
Eq.__doc__ = 'Eq'
Eq.__match_args__ = ()
NotEq.__doc__ = 'NotEq'
NotEq.__match_args__ = ()
Lt.__doc__ = 'Lt'
Lt.__match_args__ = ()
LtE.__doc__ = 'LtE'
LtE.__match_args__ = ()
Gt.__doc__ = 'Gt'
Gt.__match_args__ = ()
GtE.__doc__ = 'GtE'
GtE.__match_args__ = ()
Is.__doc__ = 'Is'
Is.__match_args__ = ()
IsNot.__doc__ = 'IsNot'
IsNot.__match_args__ = ()
In.__doc__ = 'In'
In.__match_args__ = ()
NotIn.__doc__ = 'NotIn'
NotIn.__match_args__ = ()
comprehension.__doc__ = 'comprehension(expr target, expr iter, expr* ifs, int is_async)'
comprehension.__match_args__ = ('target', 'iter', 'ifs', 'is_async')
excepthandler.__doc__ = 'excepthandler = ExceptHandler(expr? type, identifier? name, stmt* body)'
excepthandler.__match_args__ = ()
ExceptHandler.__doc__ = 'ExceptHandler(expr? type, identifier? name, stmt* body)'
ExceptHandler.__match_args__ = ('type', 'name', 'body')
arguments.__doc__ = 'arguments(arg* posonlyargs, arg* args, arg? vararg, arg* kwonlyargs, expr?* kw_defaults, arg? kwarg, expr* defaults)'
arguments.__match_args__ = ('posonlyargs', 'args', 'vararg', 'kwonlyargs', 'kw_defaults', 'kwarg', 'defaults')
arg.__doc__ = 'arg(identifier arg, expr? annotation, string? type_comment)'
arg.__match_args__ = ('arg', 'annotation', 'type_comment')
keyword.__doc__ = 'keyword(identifier? arg, expr value)'
keyword.__match_args__ = ('arg', 'value')
alias.__doc__ = 'alias(identifier name, identifier? asname)'
alias.__match_args__ = ('name', 'asname')
withitem.__doc__ = 'withitem(expr context_expr, expr? optional_vars)'
withitem.__match_args__ = ('context_expr', 'optional_vars')
match_case.__doc__ = 'match_case(pattern pattern, expr? guard, stmt* body)'
match_case.__match_args__ = ('pattern', 'guard', 'body')
pattern.__doc__ = 'pattern = MatchValue(expr value)\n        | MatchSingleton(constant value)\n        | MatchSequence(pattern* patterns)\n        | MatchMapping(expr* keys, pattern* patterns, identifier? rest)\n        | MatchClass(expr cls, pattern* patterns, identifier* kwd_attrs, pattern* kwd_patterns)\n        | MatchStar(identifier? name)\n        | MatchAs(pattern? pattern, identifier? name)\n        | MatchOr(pattern* patterns)'
pattern.__match_args__ = ()
MatchValue.__doc__ = 'MatchValue(expr value)'
MatchValue.__match_args__ = ('value',)
MatchSingleton.__doc__ = 'MatchSingleton(constant value)'
MatchSingleton.__match_args__ = ('value',)
MatchSequence.__doc__ = 'MatchSequence(pattern* patterns)'
MatchSequence.__match_args__ = ('patterns',)
MatchMapping.__doc__ = 'MatchMapping(expr* keys, pattern* patterns, identifier? rest)'
MatchMapping.__match_args__ = ('keys', 'patterns', 'rest')
MatchClass.__doc__ = 'MatchClass(expr cls, pattern* patterns, identifier* kwd_attrs, pattern* kwd_patterns)'
MatchClass.__match_args__ = ('cls', 'patterns', 'kwd_attrs', 'kwd_patterns')
MatchStar.__doc__ = 'MatchStar(identifier? name)'
MatchStar.__match_args__ = ('name',)
MatchAs.__doc__ = 'MatchAs(pattern? pattern, identifier? name)'
MatchAs.__match_args__ = ('pattern', 'name')
MatchOr.__doc__ = 'MatchOr(pattern* patterns)'
MatchOr.__match_args__ = ('patterns',)
type_ignore.__doc__ = 'type_ignore = TypeIgnore(int lineno, string tag)'
type_ignore.__match_args__ = ()
TypeIgnore.__doc__ = 'TypeIgnore(int lineno, string tag)'
TypeIgnore.__match_args__ = ('lineno', 'tag')
type_param.__doc__ = 'type_param = TypeVar(identifier name, expr? bound, expr? default_value)\n           | ParamSpec(identifier name, expr? default_value)\n           | TypeVarTuple(identifier name, expr? default_value)'
type_param.__match_args__ = ()
TypeVar.__doc__ = 'TypeVar(identifier name, expr? bound, expr? default_value)'
TypeVar.__match_args__ = ('name', 'bound', 'default_value')
ParamSpec.__doc__ = 'ParamSpec(identifier name, expr? default_value)'
ParamSpec.__match_args__ = ('name', 'default_value')
TypeVarTuple.__doc__ = 'TypeVarTuple(identifier name, expr? default_value)'
TypeVarTuple.__match_args__ = ('name', 'default_value')

_STRING_PREFIXES = ('r', 'b', 'u', 'f', 't', 'rb', 'br', 'fr', 'rf', 'tr', 'rt')

_OPS3 = ('**=', '//=', '<<=', '>>=', '...')
_OPS2 = ('**', '//', '<<', '>>', '==', '!=', '<=', '>=', ':=', '->',
         '+=', '-=', '*=', '/=', '%=', '@=', '&=', '|=', '^=')
_OPS1 = '()[]{},:;.+-*/%@=<>!~&|^'

class _Tok:
    def __init__(self, kind, text, srow, scol, erow, ecol):
        self.kind = kind
        self.text = text
        self.srow = srow
        self.scol = scol
        self.erow = erow
        self.ecol = ecol
        self.value = None
        self.prefix = ''
        self.quote = ''
        self.cs = 0
        self.ce = 0
        self.cs_row = 0
        self.cs_col = 0

def _adv(source, i, count, row, col):
    if count <= 0:
        return (i, row, col)
    stop = i + count
    chunk = source[i:stop]
    breaks = chunk.count('\n')
    if breaks:
        row += breaks
        col = len(chunk[chunk.rfind('\n') + 1:].encode('utf-8'))
    else:
        col += len(chunk.encode('utf-8'))
    return (stop, row, col)

def _string_skip(source, i):
    # i at a quote; answers the index just past the closing quote.
    n = len(source)
    q = source[i]
    if source[i:i + 3] == q * 3:
        closer = q * 3
        i += 3
    else:
        closer = q
        i += 1
    while True:
        quote = source.find(closer, i)
        if quote < 0:
            _syntax('unterminated string literal')
        escape = source.find('\\', i)
        if escape < 0 or quote < escape:
            return quote + len(closer)
        i = escape + 2

def _string_skip_pos(source, i, row, col):
    start = i
    i = _string_skip(source, i)
    _, row, col = _adv(source, start, i - start, row, col)
    return (i, row, col)

def _fstring_end(source, i):
    # i at the opening quote of an f-string literal; answers the index
    # just past the closing quote, the quote and the closing quote's
    # index. Inside a replacement field every brace is structural; only
    # in the text between them are {{ and }} escapes.
    n = len(source)
    q = source[i]
    if source[i:i + 3] == q * 3:
        quote = q * 3
    else:
        quote = q
    i += len(quote)
    depth = 0
    while i < n:
        c = source[i]
        if c == '\\':
            i += 2
            continue
        if depth == 0 and source[i:i + len(quote)] == quote:
            return (i + len(quote), quote, i)
        if depth == 0 and c == '\n' and len(quote) == 1:
            _syntax('f-string: unterminated string')
        if c == '{':
            if depth == 0 and source[i + 1:i + 2] == '{':
                i += 2
                continue
            depth += 1
            i += 1
            continue
        if c == '}':
            if depth == 0 and source[i + 1:i + 2] == '}':
                i += 2
                continue
            depth -= 1
            i += 1
            continue
        if depth > 0 and (c == '"' or c == "'"):
            i = _string_skip(source, i)
            continue
        i += 1
    _syntax('unterminated f-string literal')

def _check_single_line(source, q, after):
    # A single-quoted string literal stands on one line: a raw newline
    # inside it (one a backslash does not continue) is unterminated.
    if source[q:q + 3] == source[q] * 3:
        return
    i = q
    while i < after:
        if source[i] == '\\':
            i += 2
            continue
        if source[i] == '\n':
            _syntax('unterminated string literal')
        i += 1

def _lex(source, start, end, row, col, for_field, comments=None):
    if not isinstance(source, _Chars):
        source = _Chars(source)
    # Tokens over source[start:end], beginning at (row, col). Inside a
    # replacement field (for_field) newlines are plain whitespace; outside
    # one they end a statement unless brackets are open. When comments is
    # a list each comment is recorded there as (row, col, source text),
    # '#' first, without entering the token stream.
    toks = []
    i = start
    depth = 0
    while i < end:
        c = source[i]
        srow, scol = row, col
        if c == ' ' or c == '\t' or c == '\f':
            i += 1
            col += 1
            continue
        if c == '\r':
            i += 1
            continue
        if c == '\\' and source[i + 1:i + 2] == '\n':
            i, row, col = _adv(source, i, 2, row, col)
            continue
        if c == '\n':
            i += 1
            row += 1
            col = 0
            if depth > 0 or for_field:
                continue
            toks.append(_Tok('newline', '\n', srow, scol, srow, scol + 1))
            continue
        if c == '#' and not for_field:
            j = source.find('\n', i)
            if j < 0 or j > end:
                j = end
            if comments is not None:
                comments.append((srow, scol, source[i:j]))
            col += len(source[i:j].encode('utf-8'))
            i = j
            continue
        if c.isdigit() or (c == '.' and source[i + 1:i + 2].isdigit()):
            j = i
            while j < end and (source[j].isalnum() or source[j] in '_.'):
                if source[j] in 'eE':
                    j += 1
                    if j < end and (source[j] == '+' or source[j] == '-'):
                        j += 1
                    continue
                j += 1
            toks.append(_Tok('num', source[i:j], srow, scol, srow, scol + len(source[i:j].encode('utf-8'))))
            col += len(source[i:j].encode('utf-8'))
            i = j
            continue
        if c == '_' or c.isalpha():
            j = i
            while j < end and (source[j] == '_' or source[j].isalnum()):
                j += 1
            word = source[i:j]
            lowered = word.lower()
            if j < end and source[j] in '\'"' and lowered in _STRING_PREFIXES:
                if 'f' in lowered or 't' in lowered:
                    lit_end, quote, content_end = _fstring_end(source, j)
                    ci, crow, ccol = _adv(source, i, (j - i) + len(quote), row, col)
                    i2, erow, ecol = _adv(source, ci, lit_end - ci, crow, ccol)
                    tok = _Tok('tstr' if 't' in lowered else 'fstr', source[i:lit_end], srow, scol, erow, ecol)
                    tok.prefix = lowered
                    tok.quote = quote
                    tok.cs = ci
                    tok.ce = content_end
                    tok.cs_row = crow
                    tok.cs_col = ccol
                    toks.append(tok)
                    i, row, col = i2, erow, ecol
                    continue
                value, after = _read_string(source, j, lowered)
                _check_single_line(source, j, after)
                _, erow, ecol = _adv(source, i, after - i, row, col)
                tok = _Tok('str', source[i:after], srow, scol, erow, ecol)
                tok.value = value
                tok.prefix = lowered
                toks.append(tok)
                i, row, col = after, erow, ecol
                continue
            toks.append(_Tok('name', word, srow, scol, srow, scol + len(source[i:j].encode('utf-8'))))
            col += len(source[i:j].encode('utf-8'))
            i = j
            continue
        if c == '"' or c == "'":
            value, after = _read_string(source, i, '')
            _check_single_line(source, i, after)
            _, erow, ecol = _adv(source, i, after - i, row, col)
            tok = _Tok('str', source[i:after], srow, scol, erow, ecol)
            tok.value = value
            toks.append(tok)
            i, row, col = after, erow, ecol
            continue
        matched = None
        if source[i:i + 3] in _OPS3:
            matched = source[i:i + 3]
        elif source[i:i + 2] in _OPS2:
            matched = source[i:i + 2]
        elif c in _OPS1:
            matched = c
        if matched is None:
            _syntax('invalid syntax')
        if matched in '([{':
            depth += 1
        elif matched in ')]}':
            depth -= 1
        toks.append(_Tok('op', matched, srow, scol, srow, scol + len(matched)))
        col += len(matched)
        i += len(matched)
    toks.append(_Tok('end', '', row, col, row, col))
    return toks

def _ftext(source, s, e, raw, escapes):
    # The decoded value of one f-string text piece over source[s:e].
    # Doubled braces are escapes in a literal's text; a format spec has
    # no such escapes (a single '{' opens a nested field there instead).
    units = []
    i = s
    while i < e:
        c = source[i]
        if escapes and c == '{' and source[i + 1:i + 2] == '{':
            units.append('{')
            i += 2
            continue
        if escapes and c == '}' and source[i + 1:i + 2] == '}':
            units.append('}')
            i += 2
            continue
        if c == '\\' and i + 1 < e:
            if raw:
                units.append('\\')
                units.append(source[i + 1])
                i += 2
                continue
            piece, i = _read_escape(source, i + 1, False)
            if piece is not None:
                units.append(piece)
            continue
        units.append(c)
        i += 1
    return ''.join(units)

def _joined_values(source, seg_i, seg_end, row, col, raw, spec, multi, template=False):
    # The Constant and FormattedValue nodes of one f-string literal's
    # text over [seg_i, seg_end), in order; adjacent text pieces are not
    # merged here (merging happens only across concatenated literals).
    # In a format spec (spec) doubled braces open nested fields rather
    # than reading as escapes, and a single-quoted f-string (not multi)
    # allows no newline in its text.
    parts = []
    text_start = seg_i
    text_srow, text_scol = row, col
    i = seg_i
    while i < seg_end:
        c = source[i]
        if c == '\\':
            i, row, col = _adv(source, i, 2, row, col)
            continue
        if not spec and c == '{' and source[i + 1:i + 2] == '{':
            i, row, col = _adv(source, i, 2, row, col)
            continue
        if not spec and c == '}' and source[i + 1:i + 2] == '}':
            i, row, col = _adv(source, i, 2, row, col)
            continue
        if c == '\n' and not multi and not spec:
            _syntax('f-string: unterminated string')
        if c == '{':
            if text_start < i:
                parts.append(Constant(value=_ftext(source, text_start, i, raw, not spec),
                                      lineno=text_srow, col_offset=text_scol,
                                      end_lineno=row, end_col_offset=col))
            debug_text, fv, i, row, col = _formatted(source, i, row, col, raw, multi, template)
            if debug_text is not None:
                parts.append(Constant(value=debug_text[0], lineno=debug_text[1],
                                      col_offset=debug_text[2],
                                      end_lineno=debug_text[3],
                                      end_col_offset=debug_text[4]))
            parts.append(fv)
            text_start = i
            text_srow, text_scol = row, col
            continue
        i, row, col = _adv(source, i, 1, row, col)
    if text_start < seg_end:
        parts.append(Constant(value=_ftext(source, text_start, seg_end, raw, not spec),
                              lineno=text_srow, col_offset=text_scol,
                              end_lineno=row, end_col_offset=col))
    return parts

def _formatted(source, i, row, col, raw, multi, template=False):
    # i at the '{' opening a replacement field; answers the FormattedValue
    # node, the index just past its '}', and the position there. The
    # node covers its braces; a format spec is a JoinedStr starting at
    # the ':' and ending where the field's '}' stands. A field may span
    # lines; its spec may not unless the f-string is triple-quoted.
    srow, scol = row, col
    i, row, col = _adv(source, i, 1, row, col)
    expr_start = i
    esrow, escol = row, col
    expr_end = -1
    conv = -1
    debug = None
    debug_end = -1
    spec_start = -1
    spec_srow = 0
    spec_scol = 0
    body_srow = 0
    body_scol = 0
    depth = 0
    n = len(source)
    while True:
        if i >= n:
            _syntax("f-string: valid expression required before '}'")
        c = source[i]
        if c == '"' or c == "'":
            i, row, col = _string_skip_pos(source, i, row, col)
            continue
        if c == '\\':
            i, row, col = _adv(source, i, 2, row, col)
            continue
        if c in '([{':
            depth += 1
            i, row, col = _adv(source, i, 1, row, col)
            continue
        if c in ')]':
            depth -= 1
            i, row, col = _adv(source, i, 1, row, col)
            continue
        if c == '}':
            if depth == 0:
                if expr_end < 0:
                    expr_end = i
                if debug is not None and debug_end < 0:
                    debug_end = i
                break
            depth -= 1
            i, row, col = _adv(source, i, 1, row, col)
            continue
        if depth == 0 and c == '=' and source[i + 1:i + 2] != '=' and source[i - 1:i] not in ('=', '!', '<', '>', ':'):
            # The debug form 'expr=' repeats the expression text before it.
            if template or expr_end >= 0:
                _no_tree()
            expr_end = i
            debug = (srow, scol, row, col + 1)
        if depth == 0 and c == '!' and source[i + 1:i + 2] != '=':
            if expr_end < 0:
                expr_end = i
            if debug is not None and debug_end < 0:
                debug_end = i
            conv = ord(source[i + 1])
            i, row, col = _adv(source, i, 2, row, col)
            continue
        if depth == 0 and c == ':':
            if source[i + 1:i + 2] == '=':
                _no_tree()
            if expr_end < 0:
                expr_end = i
            if debug is not None and debug_end < 0:
                debug_end = i
            spec_srow, spec_scol = row, col
            i, row, col = _adv(source, i, 1, row, col)
            spec_start = i
            body_srow, body_scol = row, col
            sdepth = 0
            while True:
                if i >= n:
                    _syntax("f-string: valid expression required before '}'")
                c2 = source[i]
                if c2 == '"' or c2 == "'":
                    i, row, col = _string_skip_pos(source, i, row, col)
                    continue
                if c2 == '\\':
                    i, row, col = _adv(source, i, 2, row, col)
                    continue
                if c2 == '\n' and not multi:
                    _syntax('f-string: newlines are not allowed in format specifiers for single quoted f-strings')
                if c2 == '{':
                    sdepth += 1
                    i, row, col = _adv(source, i, 1, row, col)
                    continue
                if c2 == '}':
                    if sdepth == 0:
                        break
                    sdepth -= 1
                    i, row, col = _adv(source, i, 1, row, col)
                    continue
                i, row, col = _adv(source, i, 1, row, col)
            break
        i, row, col = _adv(source, i, 1, row, col)
    # i is at the field's closing '}' and (row, col) is its position.
    value = _parse_field_expr(source, expr_start, expr_end, esrow, escol)
    spec = None
    if spec_start >= 0:
        values = _joined_values(source, spec_start, i, body_srow, body_scol, raw, True, multi)
        spec = JoinedStr(values=values, lineno=spec_srow, col_offset=spec_scol,
                         end_lineno=row, end_col_offset=col)
    debug_text = None
    if debug is not None:
        if debug_end < 0:
            debug_end = i
        # A debug field reprs its value unless a format spec says otherwise.
        if spec is None and conv == -1:
            conv = 114
        debug_text = (source[expr_start:debug_end], debug[0], debug[1],
                      debug[2], debug[3])
    node = FormattedValue(value=value, conversion=conv, format_spec=spec,
                          lineno=srow, col_offset=scol,
                          end_lineno=row, end_col_offset=col + 1)
    if template:
        node = Interpolation(value=value, str=source[expr_start:expr_end].strip(),
                             conversion=conv, format_spec=spec, lineno=srow,
                             col_offset=scol, end_lineno=row, end_col_offset=col + 1)
    i, row, col = _adv(source, i, 1, row, col)
    return (debug_text, node, i, row, col)

def _parse_field_expr(source, s, e, row, col):
    toks = _lex(source, s, e, row, col, True)
    parser = _Parser(source, toks)
    node, ls, le = parser.parse_expr(True)
    if parser.peek().kind != 'end':
        _syntax('invalid syntax')
    return node

_STMT_WORDS = ('def', 'class', 'if', 'elif', 'else', 'for', 'while', 'try',
               'except', 'finally', 'with', 'return', 'yield', 'raise',
               'import', 'from', 'pass', 'break', 'continue', 'global',
               'nonlocal', 'assert', 'del', 'async', 'await', 'lambda')

_BINOPS = {'+': Add, '-': Sub, '*': Mult, '/': Div, '//': FloorDiv,
           '%': Mod, '@': MatMult, '<<': LShift, '>>': RShift,
           '|': BitOr, '^': BitXor, '&': BitAnd, '**': Pow}
_BINOP_PRECEDENCE = {'|': 1, '^': 2, '&': 3, '<<': 4, '>>': 4,
                     '+': 5, '-': 5, '*': 6, '/': 6, '//': 6, '%': 6, '@': 6}
_CMPOPS = {'==': Eq, '!=': NotEq, '<': Lt, '<=': LtE, '>': Gt, '>=': GtE}
_UNARY = {'+': UAdd, '-': USub, '~': Invert}

def _indent(message):
    raise 'IndentationError: ' + message

def _tab(message):
    raise 'TabError: ' + message

_AUG_NAMES = {Tuple: 'tuple', List: 'list', Dict: 'dict literal',
              Set: 'set display', Call: 'function call',
              BinOp: 'expression', UnaryOp: 'expression',
              BoolOp: 'expression', Compare: 'comparison',
              ListComp: 'list comprehension', Await: 'await expression',
              JoinedStr: 'f-string expression',
              FormattedValue: 'f-string expression'}

# Name an invalid augmented-assignment target in the parser diagnostic.
def _aug_target_name(node):
    name = _AUG_NAMES.get(type(node))
    if name is not None:
        return name
    if type(node) == Constant:
        value = node.value
        if value is None:
            return 'None'
        if value is True:
            return 'True'
        if value is False:
            return 'False'
        if type(value).__name__ == 'ellipsis':
            return 'ellipsis'
        return 'literal'
    return 'expression'

def _comment_payload(text):
    # What follows 'type:' in a comment (its source, '#' first, to the end
    # of the line), or None when the comment is not a '# type:' comment at
    # all. CPython's tokenizer matches '#', then any spaces and tabs, then
    # 'type:', then spaces and tabs again.
    i = 1
    n = len(text)
    while i < n and (text[i] == ' ' or text[i] == '\t'):
        i += 1
    if text[i:i + 5] != 'type:':
        return None
    i += 5
    while i < n and (text[i] == ' ' or text[i] == '\t'):
        i += 1
    return text[i:]

def _is_ignore(payload):
    # A 'type: ignore' comment is collected apart and is not a type
    # comment; 'ignore' must end there or be followed by a non-alphanumeric
    # ASCII character ('ignoreé' and 'ignorewhatever' are type comments).
    return payload[:6] == 'ignore' and (len(payload) == 6 or
                                        (ord(payload[6]) < 128 and
                                         not payload[6].isalnum()))

def _type_ignore_tag(text):
    # The tag of a 'type: ignore' comment -- whatever stands after
    # 'ignore', '[' or '=' or ' ...' included -- or None.
    payload = _comment_payload(text)
    if payload is None or not _is_ignore(payload):
        return None
    return payload[6:]

def _feature_minor(feature_version):
    # feature_version as a 3.minor pair's minor. None is the running
    # version; an int is the minor of 3.x; a pair is (major, minor).
    if feature_version is None:
        import sys
        return sys.version_info[1]
    if type(feature_version) == type(0):
        return feature_version
    major, minor = feature_version
    if major != 3:
        raise 'ValueError: feature_version major must be 3'
    return minor

class _Parser:
    def __init__(self, source, toks, optimize=-1, fold_debug=False, comments=None, fv=None):
        self.source = source
        self.toks = toks
        self.at = 0
        self.fv = _feature_minor(fv)
        self.optimize = optimize
        self.fold_debug = fold_debug
        self.lines = source.split('\n')
        self.icols = {}
        self.indent_stack = [0]
        # CPython's alternate indentation column (each tab one wide) for
        # every active level. A level's spelling rides on this stack, so
        # leaving a suite restores the enclosing spelling instead of
        # comparing one sibling suite against another that reused the
        # same column.
        self.alt_stack = [0]
        # Type comments by row, from the comments _lex recorded; a row is
        # struck out as its comment attaches to a node, and a type comment
        # left over at the end of the module is a misplaced one.
        self.tcomments = {}
        self.used = set()
        if comments is not None:
            for row, col, text in comments:
                payload = _type_comment_payload(text)
                if payload is not None:
                    self.tcomments[row] = payload

    def indent_col(self, tok):
        # A statement's indentation: the line's leading whitespace, each
        # tab advancing to the next multiple of eight, as CPython's
        # tokenizer measures it.
        row = tok.srow
        known = self.icols.get(row)
        if known is not None:
            return known
        text = self.lines[row - 1] if row - 1 < len(self.lines) else ''
        col = 0
        i = 0
        stop = tok.scol
        while i < stop and i < len(text):
            if text[i] == '\t':
                col += 8 - col % 8
            else:
                col += 1
            i += 1
        self.icols[row] = col
        return col

    def alt_indent_col(self, tok):
        # The same leading whitespace with each tab and space counting
        # one: CPython's alternate indentation column, which records how
        # a level is spelled where indent_col records how wide it is.
        text = self.lines[tok.srow - 1] if tok.srow - 1 < len(self.lines) else ''
        stop = tok.scol if tok.scol < len(text) else len(text)
        i = 0
        while i < stop and (text[i] == ' ' or text[i] == '\t' or
                            text[i] == '\f'):
            i += 1
        return i

    def prefix_len(self, tok):
        # The leading whitespace of the token's line, in characters; a
        # statement's first token always stands right after it.
        text = self.lines[tok.srow - 1] if tok.srow - 1 < len(self.lines) else ''
        i = 0
        n = len(text)
        while i < n and (text[i] == ' ' or text[i] == '\t' or text[i] == '\f'):
            i += 1
        return i

    def at_clause(self, word, parent_col):
        # A clause keyword (else, except, ...) opening a line of its own
        # at the owning statement's indentation.
        tok = self.toks[self.at]
        return (tok.kind == 'name' and tok.text == word and
                tok.scol == self.prefix_len(tok) and
                self.indent_col(tok) == parent_col)

    def take_comment(self, row):
        # The type comment standing on row, struck out so it cannot
        # attach twice; None when the row carries none.
        payload = self.tcomments.get(row)
        if payload is not None and row not in self.used:
            self.used.add(row)
            return payload
        return None

    def check_comments(self):
        for row in self.tcomments:
            if row not in self.used:
                _syntax('invalid syntax')

    def parse_func_type(self):
        # func_type mode: '(type, *type, **type) -> type'. The starred
        # forms may each appear once, the doubly starred one last.
        self.expect_op('(')
        argtypes = []
        starred = False
        dstarred = False
        while not self.at_op(')'):
            if self.at_op('*'):
                self.pop()
                if starred or dstarred:
                    _syntax('invalid syntax')
                starred = True
            elif self.at_op('**'):
                self.pop()
                if dstarred:
                    _syntax('invalid syntax')
                dstarred = True
            elif dstarred:
                _syntax('invalid syntax')
            entry, es, ee = self.parse_or()
            argtypes.append(entry)
            if self.at_op(','):
                self.pop()
                continue
            break
        self.expect_op(')')
        self.expect_op('->')
        returns, rs, re = self.parse_expr(False)
        while self.toks[self.at].kind == 'newline':
            self.pop()
        if self.toks[self.at].kind != 'end':
            _syntax('invalid syntax')
        return FunctionType(argtypes=argtypes, returns=returns)

    def peek(self):
        return self.toks[self.at]

    def pop(self):
        tok = self.toks[self.at]
        self.at += 1
        return tok

    def at_op(self, text):
        tok = self.toks[self.at]
        return tok.kind == 'op' and tok.text == text

    def at_name(self, word):
        tok = self.toks[self.at]
        return tok.kind == 'name' and tok.text == word

    def expect_op(self, text):
        if not self.at_op(text):
            _syntax('invalid syntax')
        return self.pop()

    def parse_module(self, single=False):
        self._inline_tail = 0
        body = self.parse_block(0, single)
        if single and self._inline_tail >= len(self.source.split('\n')):
            raise SyntaxError('invalid syntax')
        return Module(body=body, type_ignores=[])

    # Read a block at one indentation level, stopping at its dedent.
    def parse_block(self, indent, single=False):
        body = []
        while True:
            while self.toks[self.at].kind == 'newline':
                self.pop()
            tok = self.toks[self.at]
            if tok.kind == 'end':
                return body
            if tok.scol != self.prefix_len(tok):
                _syntax('invalid syntax')
            col = self.indent_col(tok)
            if col < indent:
                if col not in self.indent_stack:
                    _indent('unindent does not match any outer indentation level')
                return body
            alt = self.alt_indent_col(tok)
            if col > indent:
                if alt <= self.alt_stack[-1]:
                    _tab('inconsistent use of tabs and spaces in indentation')
                _indent('unexpected indent')
            if alt != self.alt_stack[-1]:
                _tab('inconsistent use of tabs and spaces in indentation')
            node = self.parse_stmt()
            body.append(node)
            while self.at_op(';'):
                self.pop()
                if self.toks[self.at].kind in ('end', 'newline'):
                    break
                item = self.parse_stmt()
                if _compound_statement(item):
                    raise SyntaxError('invalid syntax')
                body.append(item)
            if self.toks[self.at].kind not in ('end', 'newline') and self.toks[self.at].srow <= body[-1].end_lineno:
                _syntax('invalid syntax')
            if single:
                while self.toks[self.at].kind == 'newline':
                    self.pop()
                if self.toks[self.at].kind != 'end':
                    if _compound_statement(body[0]):
                        raise SyntaxError('invalid syntax')
                    raise SyntaxError('multiple statements found while compiling a single statement')
                return body

    # Read either a one-line suite or an indented suite after its colon.
    # Attach a loop/with header comment or a function's header/body-leading comment.
    def suite_comment(self, function=False):
        colon = self.toks[self.at]
        if self.toks[self.at + 1].kind != 'newline':
            return None
        found = []
        header = self.take_comment(colon.srow)
        if header is not None:
            found.append(header)
        if function:
            ahead = self.at + 1
            while self.toks[ahead].kind == 'newline':
                ahead += 1
            for row in range(colon.srow + 1, self.toks[ahead].srow):
                inner = self.take_comment(row)
                if inner is not None:
                    found.append(inner)
            if len(found) > 1:
                _syntax('Cannot have two type comments on def')
        return found[-1] if found else None

    def parse_suite(self, header):
        self.expect_op(':')
        if self.toks[self.at].kind != 'newline':
            body = [self.parse_simple_stmt()]
            while self.at_op(';'):
                self.pop()
                if self.toks[self.at].kind in ('end', 'newline'):
                    break
                item = self.parse_stmt()
                if _compound_statement(item):
                    raise SyntaxError('invalid syntax')
                body.append(item)
            self._inline_tail = body[-1].end_lineno
            return body
        while self.toks[self.at].kind == 'newline':
            self.pop()
        token = self.toks[self.at]
        if token.kind == 'end':
            _indent('expected an indented block')
        indent = self.indent_col(token)
        alt = self.alt_indent_col(token)
        parent = self.indent_col(header)
        if indent <= parent:
            if indent == parent and alt != self.alt_stack[-1]:
                _tab('inconsistent use of tabs and spaces in indentation')
            _indent('expected an indented block')
        if alt <= self.alt_stack[-1]:
            _tab('inconsistent use of tabs and spaces in indentation')
        self.indent_stack.append(indent)
        self.alt_stack.append(alt)
        body = self.parse_block(indent)
        self.indent_stack.pop()
        self.alt_stack.pop()
        self._inline_tail = 0
        return body

    # Build a located node from its first token and last syntactic element.
    def located(self, cls, first, last, **fields):
        if isinstance(last, AST):
            erow, ecol = last.end_lineno, last.end_col_offset
        else:
            erow, ecol = last.erow, last.ecol
        return cls(lineno=first.srow, col_offset=first.scol,
                   end_lineno=erow, end_col_offset=ecol, **fields)

    # Preserve parameter categories, annotations and default expressions.
    def parse_parameters(self, closing):
        posonly = []
        positional = []
        kwonly = []
        defaults = []
        kw_defaults = []
        vararg = None
        kwarg = None
        keyword_only = False
        split_positional = False
        bare_star = False
        while not self.at_op(closing):
            if kwarg is not None:
                _syntax('invalid syntax')
            argument = None
            if self.at_op('/'):
                if split_positional or keyword_only or not positional:
                    _syntax('invalid syntax')
                split_positional = True
                self.pop()
                posonly = positional
                positional = []
            else:
                category = ''
                if self.at_op('*') or self.at_op('**'):
                    category = self.pop().text
                    if category == '*' and keyword_only:
                        _syntax('invalid syntax')
                    keyword_only = True
                    if category == '*' and self.at_op(','):
                        bare_star = True
                        self.pop()
                        continue
                token = self.pop()
                if token.kind != 'name':
                    _syntax('invalid syntax')
                annotation = None
                if self.at_op(':') and closing == ')':
                    self.pop()
                    annotation, ns, ne = self.parse_expr(False)
                last = self.toks[self.at - 1]
                argument = self.located(arg, token, last, arg=token.text, annotation=annotation)
                default = None
                has_default = self.at_op('=')
                if has_default:
                    if category in ('*', '**'):
                        _syntax('invalid syntax')
                    self.pop()
                    default, ds, de = self.parse_expr(False)
                if category == '**':
                    kwarg = argument
                elif category == '*':
                    vararg = argument
                elif keyword_only:
                    kwonly.append(argument)
                    kw_defaults.append(default)
                else:
                    if not has_default and defaults:
                        _syntax('non-default argument follows default argument')
                    positional.append(argument)
                    if has_default:
                        defaults.append(default)
            more = self.at_op(',')
            row = self.pop().srow if more else self.toks[self.at - 1].erow
            if argument is not None:
                argument.type_comment = self.take_comment(row)
            if not more:
                break
        if bare_star and not kwonly:
            _syntax('named arguments must follow bare *')
        return arguments(posonlyargs=posonly, args=positional, vararg=vararg,
                         kwonlyargs=kwonly, kw_defaults=kw_defaults, kwarg=kwarg, defaults=defaults)

    # Read PEP 695 and PEP 696 parameter declarations from their source tokens.
    def parse_type_params(self):
        params = []
        if not self.at_op('['):
            return params
        self.pop()
        while not self.at_op(']'):
            first = self.toks[self.at]
            kind = TypeVar
            if self.at_op('*') or self.at_op('**'):
                kind = ParamSpec if self.pop().text == '**' else TypeVarTuple
            name = self.pop()
            if name.kind != 'name':
                _syntax('invalid syntax')
            bound = None
            default = None
            if self.at_op(':'):
                self.pop()
                bound, bs, be = self.parse_expr(False)
            if self.at_op('='):
                self.pop()
                default, ds, de = self.parse_expr(False)
            fields = {'name': name.text, 'default_value': default}
            if kind is TypeVar:
                fields['bound'] = bound
            elif bound is not None:
                _syntax('invalid syntax')
            params.append(self.located(kind, first, self.toks[self.at - 1], **fields))
            if not self.at_op(','):
                break
            self.pop()
        self.expect_op(']')
        return params

    # Convert pattern grammar atoms while retaining their own source positions.
    def pattern_from_expr(self, node):
        if isinstance(node, Name):
            pattern = MatchAs(name=None if node.id == '_' else node.id)
        elif isinstance(node, Constant):
            pattern = MatchSingleton(value=node.value) if node.value is None or type(node.value) is bool else MatchValue(value=node)
        elif isinstance(node, Attribute) or isinstance(node, UnaryOp):
            pattern = MatchValue(value=node)
        elif isinstance(node, BinOp):
            if isinstance(node.op, BitOr):
                left = self.pattern_from_expr(node.left)
                right = self.pattern_from_expr(node.right)
                alternatives = left.patterns if isinstance(left, MatchOr) else [left]
                pattern = MatchOr(patterns=alternatives + [right])
            else:
                pattern = MatchValue(value=node)
        elif isinstance(node, (Tuple, List)):
            pattern = MatchSequence(patterns=[self.pattern_from_expr(item) for item in node.elts])
        elif isinstance(node, Starred) and isinstance(node.value, Name):
            pattern = MatchStar(name=None if node.value.id == '_' else node.value.id)
        elif isinstance(node, Dict):
            keys, patterns = [], []
            rest = None
            for key, value in zip(node.keys, node.values):
                if key is None:
                    if not isinstance(value, Name) or value.id == '_' or rest is not None:
                        _syntax('invalid syntax')
                    rest = value.id
                else:
                    keys.append(key)
                    patterns.append(self.pattern_from_expr(value))
            pattern = MatchMapping(keys=keys, patterns=patterns, rest=rest)
        elif isinstance(node, Call):
            pattern = MatchClass(cls=node.func, patterns=[self.pattern_from_expr(item) for item in node.args],
                                 kwd_attrs=[item.arg for item in node.keywords],
                                 kwd_patterns=[self.pattern_from_expr(item.value) for item in node.keywords])
        else:
            _syntax('invalid syntax')
        return copy_location(pattern, node)

    # Parse statement grammar independently from the tree consumers.
    def parse_stmt(self, start=None):
        token = self.toks[self.at]
        first = token if start is None else start
        word = token.text
        if word == 'async' and self.fv >= 5:
            self.pop()
            if self.toks[self.at].text not in ('def', 'for', 'with'):
                _syntax('invalid syntax')
            node = self.parse_stmt(first)
            mapping = {FunctionDef: AsyncFunctionDef, For: AsyncFor, With: AsyncWith}
            if type(node) not in mapping:
                _syntax('invalid syntax')
            return self.located(mapping[type(node)], first, node, **dict(iter_fields(node)))
        if self.at_op('@'):
            decorators = []
            while self.at_op('@'):
                self.pop()
                decorator, ds, de = self.parse_expr(False)
                decorators.append(decorator)
                if self.toks[self.at].kind != 'newline':
                    _syntax('invalid syntax')
                # A comment between decorators is its own newline token.
                while self.toks[self.at].kind == 'newline':
                    self.pop()
            node = self.parse_stmt()
            if not isinstance(node, (FunctionDef, AsyncFunctionDef, ClassDef)):
                _syntax('invalid syntax')
            node.decorator_list = decorators
            return node
        if word == 'def':
            self.pop()
            name = self.pop()
            if name.kind != 'name':
                _syntax('invalid syntax')
            type_params = self.parse_type_params()
            self.expect_op('(')
            params = self.parse_parameters(')')
            self.expect_op(')')
            returns = None
            if self.at_op('->'):
                self.pop()
                returns, rs, re = self.parse_expr(False)
            comment = self.suite_comment(True)
            body = self.parse_suite(first)
            return self.located(FunctionDef, first, body[-1], name=name.text, args=params, body=body, returns=returns, type_params=type_params, type_comment=comment)
        if word == 'class':
            self.pop()
            name = self.pop()
            if name.kind != 'name':
                _syntax('invalid syntax')
            type_params = self.parse_type_params()
            bases, keywords = [], []
            if self.at_op('('):
                bases, keywords, close = self.parse_call()
            body = self.parse_suite(first)
            return self.located(ClassDef, first, body[-1], name=name.text, bases=bases, keywords=keywords, body=body, type_params=type_params)
        if word == 'type' and self.toks[self.at + 1].kind == 'name':
            self.pop()
            token = self.pop()
            name = self.located(Name, token, token, id=token.text, ctx=Store())
            type_params = self.parse_type_params()
            self.expect_op('=')
            value, vs, ve = self.parse_expr(False)
            return self.located(TypeAlias, first, self.toks[self.at - 1], name=name, type_params=type_params, value=value)
        if word in ('if', 'elif', 'while'):
            self.pop()
            test, ts, te = self.parse_expr(False)
            body = self.parse_suite(first)
            other = []
            while self.toks[self.at].kind == 'newline':
                self.pop()
            if self.at_clause('elif', self.indent_col(first)) and word != 'while':
                other = [self.parse_stmt()]
            elif self.at_clause('else', self.indent_col(first)):
                header = self.pop()
                other = self.parse_suite(header)
            return self.located(While if word == 'while' else If, first, (other or body)[-1], test=test, body=body, orelse=other)
        if word == 'for':
            self.pop()
            target, ts, te = self.parse_target()
            if not self.at_name('in'):
                _syntax('invalid syntax')
            self.pop()
            value, vs, ve = self.parse_expr(True)
            self.set_store(target)
            comment = self.suite_comment()
            body = self.parse_suite(first)
            other = []
            while self.toks[self.at].kind == 'newline':
                self.pop()
            if self.at_clause('else', self.indent_col(first)):
                other = self.parse_suite(self.pop())
            return self.located(For, first, (other or body)[-1], target=target, iter=value, body=body, orelse=other, type_comment=comment)
        if word == 'with':
            self.pop()
            parenthesized = self.at_op('(')
            if parenthesized:
                self.pop()
            items = []
            while True:
                context, cs, ce = self.parse_expr(False)
                target = None
                if self.at_name('as'):
                    self.pop()
                    target, ts, te = self.parse_postfix()
                    self.set_store(target)
                items.append(withitem(context_expr=context, optional_vars=target))
                if not self.at_op(','):
                    break
                self.pop()
                if parenthesized and self.at_op(')'):
                    break
            if parenthesized:
                self.expect_op(')')
            comment = self.suite_comment()
            body = self.parse_suite(first)
            return self.located(With, first, body[-1], items=items, body=body, type_comment=comment)
        if word == 'match' and self.toks[self.at + 1].text not in ('=', '(', '.', '[', ':', ';', ','):
            self.pop()
            subject, ss, se = self.parse_expr(True)
            self.expect_op(':')
            while self.toks[self.at].kind == 'newline':
                self.pop()
            indent = self.toks[self.at].scol
            if indent <= first.scol:
                raise IndentationError('expected an indented block')
            # The case clauses sit at a level of their own between the
            # subject and each body; register it so that a body's dedent
            # back to the next case is a step the reader knows.
            self.indent_stack.append(indent)
            self.alt_stack.append(self.alt_indent_col(self.toks[self.at]))
            cases = []
            while self.at_name('case') and self.toks[self.at].scol == indent:
                header = self.pop()
                pattern, vs, ve = self.parse_case_as()
                if isinstance(pattern, MatchStar) and not self.at_op(','):
                    _syntax('invalid syntax')
                if self.at_op(','):
                    elts = [pattern]
                    while self.at_op(','):
                        comma = self.pop()
                        ve = (comma.erow, comma.ecol)
                        if self.at_op(':') or self.at_name('if'):
                            break
                        elt, es, ee = self.parse_case_as()
                        elts.append(elt)
                        ve = ee
                    pattern = MatchSequence(patterns=elts, lineno=vs[0], col_offset=vs[1],
                                            end_lineno=ve[0], end_col_offset=ve[1])
                guard = None
                if self.at_name('if'):
                    self.pop()
                    guard, gs, ge = self.parse_expr(False)
                body = self.parse_suite(header)
                cases.append(match_case(pattern=pattern, guard=guard, body=body))
                while self.toks[self.at].kind == 'newline':
                    self.pop()
            self.indent_stack.pop()
            self.alt_stack.pop()
            if not cases:
                _syntax('invalid syntax')
            return self.located(Match, first, cases[-1].body[-1], subject=subject, cases=cases)
        if word == 'try':
            self.pop()
            body = self.parse_suite(first)
            handlers = []
            other = []
            finalbody = []
            starred = False
            while True:
                while self.toks[self.at].kind == 'newline':
                    self.pop()
                if self.indent_col(self.toks[self.at]) != self.indent_col(first):
                    break
                if self.at_name('except'):
                    header = self.pop()
                    if self.at_op('*'):
                        self.pop()
                        starred = True
                    typ = None
                    name = None
                    if not self.at_op(':'):
                        typ, ts, te = self.parse_expr(True)
                    if self.at_name('as'):
                        self.pop()
                        name = self.pop().text
                    suite = self.parse_suite(header)
                    handlers.append(self.located(ExceptHandler, header, suite[-1], type=typ, name=name, body=suite))
                elif self.at_name('else'):
                    other = self.parse_suite(self.pop())
                elif self.at_name('finally'):
                    finalbody = self.parse_suite(self.pop())
                    break
                else:
                    break
            if not handlers and not finalbody:
                _syntax("expected 'except' or 'finally' block")
            last = (finalbody or other or (handlers[-1].body if handlers else body))[-1]
            return self.located(TryStar if starred else Try, first, last, body=body, handlers=handlers, orelse=other, finalbody=finalbody)
        if word in ('pass', 'break', 'continue'):
            self.pop()
            return self.located({'pass': Pass, 'break': Break, 'continue': Continue}[word], first, first)
        if word in ('global', 'nonlocal'):
            self.pop()
            names = [self.pop().text]
            while self.at_op(','):
                self.pop()
                names.append(self.pop().text)
            return self.located(Global if word == 'global' else Nonlocal, first, self.toks[self.at - 1], names=names)
        if word in ('import', 'from'):
            self.pop()
            module = None
            level = 0
            if word == 'from':
                while self.at_op('.') or self.at_op('...'):
                    level += len(self.pop().text)
                parts = []
                if not self.at_name('import'):
                    parts.append(self.pop().text)
                    while self.at_op('.'):
                        self.pop()
                        parts.append(self.pop().text)
                module = '.'.join(parts) if parts else None
                if not self.at_name('import'):
                    _syntax('invalid syntax')
                self.pop()
            parens = self.at_op('(')
            if parens:
                self.pop()
            names = []
            while True:
                token = self.pop()
                parts = [token.text]
                while self.at_op('.'):
                    self.pop()
                    parts.append(self.pop().text)
                asname = None
                if self.at_name('as'):
                    self.pop()
                    asname = self.pop().text
                names.append(self.located(alias, token, self.toks[self.at - 1], name='.'.join(parts), asname=asname))
                if not self.at_op(','):
                    break
                self.pop()
                if parens and self.at_op(')'):
                    break
            if parens:
                self.expect_op(')')
            if word == 'import':
                return self.located(Import, first, self.toks[self.at - 1], names=names)
            return self.located(ImportFrom, first, self.toks[self.at - 1], module=module, names=names, level=level)
        if word in ('return', 'raise', 'assert', 'del'):
            self.pop()
            value = None
            if self.toks[self.at].kind not in ('end', 'newline') and not self.at_op(';'):
                value, vs, ve = self.parse_expr(word not in ('assert', 'raise'))
            if word == 'return':
                return self.located(Return, first, self.toks[self.at - 1], value=value)
            if word == 'raise':
                cause = None
                if self.at_name('from'):
                    self.pop()
                    cause, cs, ce = self.parse_expr(False)
                return self.located(Raise, first, self.toks[self.at - 1], exc=value, cause=cause)
            if word == 'assert':
                message = None
                if self.at_op(','):
                    self.pop()
                    message, ms, me = self.parse_expr(False)
                return self.located(Assert, first, self.toks[self.at - 1], test=value, msg=message)
            targets = value.elts if isinstance(value, Tuple) else [value]
            for target in targets:
                self.set_store(target, Del)
            return self.located(Delete, first, self.toks[self.at - 1], targets=targets)
        if first.kind == 'name' and word in _STMT_WORDS and word not in ('yield', 'await', 'lambda') and not (self.fv < 5 and word == 'async'):
            _no_tree()
        node, ls, le = self.parse_expr(True)
        if self.at_op(':'):
            self.pop()
            annotation, ns, ne = self.parse_expr(False)
            value = None
            if self.at_op('='):
                self.pop()
                value, vs, ve = self.parse_expr(False)
            self.set_store(node)
            return self.located(AnnAssign, first, self.toks[self.at - 1], target=node, annotation=annotation, value=value, simple=int(isinstance(node, Name) and node.col_offset == first.scol))
        operator = self.toks[self.at].text
        if self.toks[self.at].kind == 'op' and operator.endswith('=') and operator[:-1] in _BINOPS:
            self.pop()
            if type(node) not in (Name, Attribute, Subscript):
                _syntax("'" + _aug_target_name(node) + "' is an illegal expression for augmented assignment")
            value, vs, ve = self.parse_expr(True)
            self.set_store(node)
            return self.located(AugAssign, first, self.toks[self.at - 1], target=node, op=_BINOPS[operator[:-1]](), value=value)
        if self.at_op('='):
            targets = [node]
            while self.at_op('='):
                self.pop()
                value, vs, ve = self.parse_expr(True)
                if self.at_op('='):
                    targets.append(value)
            for target in targets:
                self.set_store(target)
            return self.located(Assign, first, self.toks[self.at - 1], targets=targets, value=value, type_comment=self.take_comment(ve[0]))
        return self.located(Expr, first, self.toks[self.at - 1], value=node)

    def expect_name(self, word):
        tok = self.toks[self.at]
        if tok.kind != 'name' or tok.text != word:
            _syntax('invalid syntax')
        return self.pop()

    def tail_of(self, body, fallback):
        if body:
            last = body[-1]
            if getattr(last, 'end_lineno', None) is not None:
                return (last.end_lineno, last.end_col_offset)
        return fallback

    def parse_simple_stmt(self):
        token = self.toks[self.at]
        if token.kind == 'name' and token.text in ('def', 'class', 'if', 'for', 'while', 'try', 'with', 'async', 'match'):
            _syntax('invalid syntax')
        return self.parse_stmt()


    def parse_if(self):
        tok = self.expect_name('if')
        test, ts, te = self.parse_expr(False)
        self.expect_op(':')
        body = self.parse_suite(tok.scol)
        orelse = self.parse_else(tok.scol)
        end = self.tail_of(orelse or body, te)
        return If(test=test, body=body, orelse=orelse,
                  lineno=tok.srow, col_offset=tok.scol,
                  end_lineno=end[0], end_col_offset=end[1])

    def parse_else(self, parent_col):
        while self.toks[self.at].kind == 'newline':
            self.pop()
        tok = self.toks[self.at]
        if self.at_name('elif') and tok.scol == parent_col:
            start = self.pop()
            test, ts, te = self.parse_expr(False)
            self.expect_op(':')
            body = self.parse_suite(parent_col)
            orelse = self.parse_else(parent_col)
            end = self.tail_of(orelse or body, te)
            return [If(test=test, body=body, orelse=orelse,
                       lineno=start.srow, col_offset=start.scol,
                       end_lineno=end[0], end_col_offset=end[1])]
        if self.at_name('else') and tok.scol == parent_col:
            self.pop()
            self.expect_op(':')
            return self.parse_suite(parent_col)
        return []

    def parse_try(self):
        tok = self.expect_name('try')
        self.expect_op(':')
        body = self.parse_suite(tok.scol)
        handlers = []
        orelse = []
        finalbody = []
        while True:
            while self.toks[self.at].kind == 'newline':
                self.pop()
            head = self.toks[self.at]
            if self.at_name('except') and head.scol == tok.scol:
                handlers.append(self.parse_except(tok.scol))
                continue
            break
        while self.toks[self.at].kind == 'newline':
            self.pop()
        if self.at_name('else') and self.toks[self.at].scol == tok.scol:
            self.pop()
            self.expect_op(':')
            orelse = self.parse_suite(tok.scol)
            while self.toks[self.at].kind == 'newline':
                self.pop()
        if self.at_name('finally') and self.toks[self.at].scol == tok.scol:
            self.pop()
            self.expect_op(':')
            finalbody = self.parse_suite(tok.scol)
        if not handlers and not orelse and not finalbody:
            _syntax('invalid syntax')
        end = self.tail_of(finalbody or orelse or body, (tok.erow, tok.ecol))
        return Try(body=body, handlers=handlers, orelse=orelse,
                   finalbody=finalbody, lineno=tok.srow, col_offset=tok.scol,
                   end_lineno=end[0], end_col_offset=end[1])

    def parse_except(self, parent_col):
        tok = self.expect_name('except')
        type_ = None
        name = None
        if not self.at_op(':'):
            type_, ts, te = self.parse_expr(False)
            if self.at_name('as'):
                self.pop()
                word = self.pop()
                if word.kind != 'name':
                    _syntax('invalid syntax')
                name = word.text
        self.expect_op(':')
        body = self.parse_suite(parent_col)
        end = self.tail_of(body, (tok.erow, tok.ecol))
        return ExceptHandler(type=type_, name=name, body=body,
                             lineno=tok.srow, col_offset=tok.scol,
                             end_lineno=end[0], end_col_offset=end[1])

    def parse_assert(self):
        tok = self.expect_name('assert')
        test, ts, te = self.parse_or()
        msg = None
        if self.at_op(','):
            self.pop()
            msg, ms, te = self.parse_or()
        return Assert(test=test, msg=msg, lineno=tok.srow,
                      col_offset=tok.scol, end_lineno=te[0],
                      end_col_offset=te[1])

    def parse_return(self):
        tok = self.expect_name('return')
        value = None
        le = (tok.erow, tok.ecol)
        if self.toks[self.at].kind not in ('newline', 'end') and not self.at_op(';'):
            value, vs, le = self.parse_expr(True)
        return Return(value=value, lineno=tok.srow, col_offset=tok.scol,
                      end_lineno=le[0], end_col_offset=le[1])

    def parse_def(self):
        tok = self.expect_name('def')
        word = self.pop()
        if word.kind != 'name':
            _syntax('invalid syntax')
        name = word.text
        self.expect_op('(')
        args = self.parse_params()
        self.expect_op(')')
        returns = None
        if self.at_op('->'):
            self.pop()
            returns, rs, re = self.parse_or()
        self.expect_op(':')
        body = self.parse_suite(tok.scol)
        end = self.tail_of(body, (tok.erow, tok.ecol))
        return FunctionDef(name=name, args=args, body=body,
                           decorator_list=[], returns=returns,
                           type_comment=None, type_params=[],
                           lineno=tok.srow, col_offset=tok.scol,
                           end_lineno=end[0], end_col_offset=end[1])

    def parse_params(self):
        posonlyargs = []
        args = []
        kwonlyargs = []
        kw_defaults = []
        defaults = []
        vararg = None
        kwarg = None
        current = args
        seen_star = False
        while not self.at_op(')'):
            if self.at_op('/'):
                self.pop()
                posonlyargs = current
                args = []
                current = args
                if self.at_op(','):
                    self.pop()
                    continue
                break
            if self.at_op('*') or self.at_op('**'):
                star = self.pop()
                if star.text == '**':
                    word = self.pop()
                    if word.kind != 'name':
                        _syntax('invalid syntax')
                    kwarg = arg(arg=word.text, annotation=None,
                                type_comment=None)
                    if self.at_op(','):
                        self.pop()
                        continue
                    break
                seen_star = True
                current = kwonlyargs
                if self.toks[self.at].kind == 'name' and self.toks[self.at].text not in _STMT_WORDS:
                    word = self.pop()
                    vararg = arg(arg=word.text, annotation=None,
                                 type_comment=None)
                if self.at_op(','):
                    self.pop()
                    continue
                break
            word = self.pop()
            if word.kind != 'name' or word.text in _STMT_WORDS:
                _syntax('invalid syntax')
            node = arg(arg=word.text, annotation=None, type_comment=None,
                       lineno=word.srow, col_offset=word.scol,
                       end_lineno=word.erow, end_col_offset=word.ecol)
            if self.at_op(':'):
                self.pop()
                node.annotation = self.parse_or()[0]
            default = None
            if self.at_op('='):
                self.pop()
                default = self.parse_expr(False)[0]
            if seen_star:
                kwonlyargs.append(node)
                kw_defaults.append(default)
            else:
                current.append(node)
                if default is not None:
                    defaults.append(default)
            if self.at_op(','):
                self.pop()
                continue
            break
        return arguments(posonlyargs=posonlyargs, args=args, vararg=vararg,
                         kwonlyargs=kwonlyargs, kw_defaults=kw_defaults,
                         kwarg=kwarg, defaults=defaults)

    # Read assignment targets without treating the following 'in' as comparison.
    def parse_target(self):
        node, ls, le = self.parse_postfix()
        if self.at_op(','):
            elts = [node]
            while self.at_op(','):
                comma = self.pop()
                if self.at_name('in') or self.at_op(')'):
                    le = (comma.erow, comma.ecol)
                    break
                item, _, le = self.parse_postfix()
                elts.append(item)
            node = Tuple(elts=elts, ctx=Load(), lineno=ls[0], col_offset=ls[1], end_lineno=le[0], end_col_offset=le[1])
        return node, ls, le

    def set_store(self, node, context=Store):
        if type(node) == Name or type(node) == Attribute or type(node) == Subscript:
            node.ctx = context()
            return
        if type(node) == Tuple or type(node) == List:
            node.ctx = context()
            for elt in node.elts:
                self.set_store(elt, context)
            return
        _syntax('invalid syntax')

    # Every level answers (node, loose start, loose end): the loose span
    # reaches over parentheses the node itself does not keep, which is
    # where statements and binary operations take their ends from.

    def parse_expr(self, bare_tuple):
        node, ls, le = self.parse_or()
        if self.at_op(':='):
            if not isinstance(node, Name):
                _syntax('cannot use assignment expressions with ' + type(node).__name__.lower())
            self.pop()
            value, vs, ve = self.parse_expr(False)
            self.set_store(node)
            node = NamedExpr(target=node, value=value, lineno=ls[0], col_offset=ls[1], end_lineno=ve[0], end_col_offset=ve[1])
            le = ve
        if self.at_name('if'):
            self.pop()
            test, ts, te = self.parse_or()
            if not self.at_name('else'):
                _syntax("expected 'else' after 'if' expression")
            self.pop()
            other, os, oe = self.parse_expr(False)
            node = IfExp(test=test, body=node, orelse=other, lineno=ls[0],
                         col_offset=ls[1], end_lineno=oe[0], end_col_offset=oe[1])
            le = oe
        if bare_tuple and self.at_op(','):
            elts = [node]
            while self.at_op(','):
                comma = self.pop()
                le = (comma.erow, comma.ecol)
                tok = self.toks[self.at]
                if tok.kind in ('newline', 'end') or self.at_op(';') or self.at_op('=') or self.at_op(')') or self.at_op(']') or self.at_op('}'):
                    break
                elt, es, ee = self.parse_expr(False)
                elts.append(elt)
                le = ee
            node = Tuple(elts=elts, ctx=Load(), lineno=ls[0], col_offset=ls[1],
                         end_lineno=le[0], end_col_offset=le[1])
        return (node, ls, le)

    def parse_case_as(self):
        value, vs, ve = self.parse_case_item()
        pattern = self.pattern_from_expr(value)
        if self.at_name('as'):
            if isinstance(pattern, MatchStar):
                _syntax('invalid syntax')
            self.pop()
            name = self.pop()
            pattern = MatchAs(pattern=pattern, name=name.text, lineno=vs[0], col_offset=vs[1],
                              end_lineno=name.erow, end_col_offset=name.ecol)
            ve = (name.erow, name.ecol)
        return (pattern, vs, ve)

    def parse_case_item(self):
        if not self.at_op('*'):
            return self.parse_or()
        star = self.pop()
        value, vs, ve = self.parse_or()
        return (Starred(value=value, ctx=Load(), lineno=star.srow, col_offset=star.scol,
                        end_lineno=ve[0], end_col_offset=ve[1]), (star.srow, star.scol), ve)

    def parse_or(self):
        node, ls, le = self.parse_and()
        while True:
            tok = self.toks[self.at]
            if tok.kind != 'name' or tok.text != 'or':
                return (node, ls, le)
            self.pop()
            right, rs, re = self.parse_and()
            if type(node) == BoolOp and type(node.op) == Or:
                node.values.append(right)
                node.end_lineno = re[0]
                node.end_col_offset = re[1]
            else:
                node = BoolOp(op=Or(), values=[node, right], lineno=ls[0],
                              col_offset=ls[1], end_lineno=re[0],
                              end_col_offset=re[1])
            le = re
        return (node, ls, le)

    def parse_and(self):
        node, ls, le = self.parse_not()
        while True:
            tok = self.toks[self.at]
            if tok.kind != 'name' or tok.text != 'and':
                return (node, ls, le)
            self.pop()
            right, rs, re = self.parse_not()
            if type(node) == BoolOp and type(node.op) == And:
                node.values.append(right)
                node.end_lineno = re[0]
                node.end_col_offset = re[1]
            else:
                node = BoolOp(op=And(), values=[node, right], lineno=ls[0],
                              col_offset=ls[1], end_lineno=re[0],
                              end_col_offset=re[1])
            le = re
        return (node, ls, le)

    def parse_not(self):
        tok = self.toks[self.at]
        if tok.kind == 'name' and tok.text == 'not':
            self.pop()
            operand, os, oe = self.parse_not()
            node = UnaryOp(op=Not(), operand=operand, lineno=tok.srow,
                           col_offset=tok.scol, end_lineno=oe[0],
                           end_col_offset=oe[1])
            return (node, (tok.srow, tok.scol), oe)
        return self.parse_comparison()

    def parse_comparison(self):
        node, ls, le = self.parse_bitor()
        ops = []
        comparators = []
        while True:
            tok = self.toks[self.at]
            op = None
            if tok.kind == 'op' and tok.text in _CMPOPS:
                self.pop()
                op = _CMPOPS[tok.text]()
            elif self.at_name('is'):
                self.pop()
                if self.at_name('not'):
                    self.pop()
                    op = IsNot()
                else:
                    op = Is()
            elif self.at_name('in'):
                self.pop()
                op = In()
            elif self.at_name('not'):
                self.pop()
                if not self.at_name('in'):
                    _syntax('invalid syntax')
                self.pop()
                op = NotIn()
            if op is None:
                break
            right, rs, le = self.parse_bitor()
            ops.append(op)
            comparators.append(right)
        if not ops:
            return (node, ls, le)
        node = Compare(left=node, ops=ops, comparators=comparators,
                       lineno=ls[0], col_offset=ls[1],
                       end_lineno=le[0], end_col_offset=le[1])
        return (node, ls, le)

    # Climb occupied precedence levels, retaining left associativity and spans.
    def parse_bitor(self, minimum=1):
        node, ls, le = self.parse_unary()
        while True:
            tok = self.toks[self.at]
            priority = _BINOP_PRECEDENCE.get(tok.text, 0) if tok.kind == 'op' else 0
            if priority < minimum:
                return node, ls, le
            if tok.text == '@' and self.fv < 5:
                _syntax("The '@' operator is only supported in Python 3.5 and greater")
            self.pop()
            right, rs, re = self.parse_bitor(priority + 1)
            node = BinOp(left=node, op=_BINOPS[tok.text](), right=right,
                         lineno=ls[0], col_offset=ls[1],
                         end_lineno=re[0], end_col_offset=re[1])
            le = re

    def parse_unary(self):
        tok = self.toks[self.at]
        if tok.kind == 'op' and tok.text in _UNARY:
            self.pop()
            operand, os, oe = self.parse_unary()
            node = UnaryOp(op=_UNARY[tok.text](), operand=operand,
                           lineno=tok.srow, col_offset=tok.scol,
                           end_lineno=oe[0], end_col_offset=oe[1])
            return (node, (tok.srow, tok.scol), oe)
        return self.parse_power()

    def parse_power(self):
        tok = self.toks[self.at]
        if tok.kind == 'name' and tok.text == 'await' and self.fv >= 5:
            # await takes a primary alone: 'await f() ** 2' squares the
            # awaited value, and 'await -x' is no expression at all.
            self.pop()
            operand, os, oe = self.parse_postfix()
            node = Await(value=operand, lineno=tok.srow, col_offset=tok.scol,
                         end_lineno=oe[0], end_col_offset=oe[1])
            ls = (tok.srow, tok.scol)
            le = oe
        else:
            node, ls, le = self.parse_postfix()
        if self.at_op('**'):
            self.pop()
            right, rs, re = self.parse_unary()
            node = BinOp(left=node, op=Pow(), right=right, lineno=ls[0],
                         col_offset=ls[1], end_lineno=re[0],
                         end_col_offset=re[1])
            le = re
        return (node, ls, le)

    # Read call arguments for both expressions and class bases.
    def parse_call(self):
        call_open = self.pop()
        args = []
        keywords = []
        while not self.at_op(')'):
            opening = self.toks[self.at]
            if self.at_op('**'):
                self.pop()
                value, vs, ve = self.parse_expr(False)
                keywords.append(keyword(arg=None, value=value, lineno=opening.srow,
                                        col_offset=opening.scol, end_lineno=ve[0], end_col_offset=ve[1]))
            elif self.at_op('*'):
                self.pop()
                value, vs, ve = self.parse_expr(False)
                args.append(Starred(value=value, ctx=Load(), lineno=opening.srow,
                                    col_offset=opening.scol, end_lineno=ve[0], end_col_offset=ve[1]))
            elif opening.kind == 'name' and self.toks[self.at + 1].text == '=':
                self.pop()
                self.pop()
                value, vs, ve = self.parse_expr(False)
                if any(k.arg == opening.text for k in keywords):
                    _syntax('keyword argument repeated: ' + opening.text)
                keywords.append(keyword(arg=opening.text, value=value, lineno=opening.srow,
                                        col_offset=opening.scol, end_lineno=ve[0], end_col_offset=ve[1]))
            else:
                if keywords:
                    _syntax('positional argument follows keyword argument')
                value, vs, ve = self.parse_expr(False)
                args.append(value)
            if self.at_name('for') or self.at_name('async'):
                generators = self.parse_generators()
                close = self.toks[self.at]
                if close.text != ')' or len(args) != 1 or keywords:
                    _syntax('Generator expression must be parenthesized')
                args[0] = self.located(GeneratorExp, call_open, close, elt=args[0], generators=generators)
            if not self.at_op(','):
                break
            self.pop()
        close_tok = self.expect_op(')')
        return args, keywords, close_tok

    def parse_postfix(self):
        node, ls, le = self.parse_atom()
        while True:
            tok = self.toks[self.at]
            if tok.kind == 'op' and tok.text == '(':
                args, keywords, close_tok = self.parse_call()
                node = Call(func=node, args=args, keywords=keywords, lineno=ls[0],
                            col_offset=ls[1], end_lineno=close_tok.erow,
                            end_col_offset=close_tok.ecol)
                le = (close_tok.erow, close_tok.ecol)
                continue
            if tok.kind == 'op' and tok.text == '[':
                self.pop()
                indices = []
                trailing = False
                while not self.at_op(']'):
                    opening = self.toks[self.at]
                    lower = None
                    if not self.at_op(':'):
                        lower, ss, se = self.parse_expr(False)
                    if self.at_op(':'):
                        colon = self.pop()
                        upper = None
                        step = None
                        last = colon
                        if not self.at_op(':') and not self.at_op(',') and not self.at_op(']'):
                            upper, us, ue = self.parse_expr(False)
                            last = self.toks[self.at - 1]
                        if self.at_op(':'):
                            last = self.pop()
                            if not self.at_op(',') and not self.at_op(']'):
                                step, ts, te = self.parse_expr(False)
                                last = self.toks[self.at - 1]
                        index = Slice(lower=lower, upper=upper, step=step,
                                      lineno=opening.srow, col_offset=opening.scol,
                                      end_lineno=last.erow, end_col_offset=last.ecol)
                    else:
                        index = lower
                    indices.append(index)
                    if not self.at_op(','):
                        break
                    self.pop()
                    trailing = True
                if not indices:
                    _syntax('invalid syntax')
                index = indices[0]
                if len(indices) > 1 or trailing or isinstance(index, Starred):
                    first = indices[0]
                    last = indices[-1]
                    index = Tuple(elts=indices, ctx=Load(), lineno=first.lineno,
                                  col_offset=first.col_offset, end_lineno=last.end_lineno,
                                  end_col_offset=self.toks[self.at - 1].ecol)
                close_tok = self.expect_op(']')
                node = Subscript(value=node, slice=index, ctx=Load(),
                                 lineno=ls[0], col_offset=ls[1],
                                 end_lineno=close_tok.erow, end_col_offset=close_tok.ecol)
                le = (close_tok.erow, close_tok.ecol)
                continue
            if tok.kind == 'op' and tok.text == '.':
                self.pop()
                name_tok = self.toks[self.at]
                if name_tok.kind != 'name':
                    _syntax('invalid syntax')
                self.pop()
                node = Attribute(value=node, attr=name_tok.text, ctx=Load(),
                                 lineno=ls[0], col_offset=ls[1],
                                 end_lineno=name_tok.erow, end_col_offset=name_tok.ecol)
                le = (name_tok.erow, name_tok.ecol)
                continue
            return (node, ls, le)

    # Read chained comprehension generators and filters in grammar order.
    def parse_generators(self):
        generators = []
        while self.at_name('for') or self.at_name('async'):
            asynchronous = int(self.at_name('async'))
            if asynchronous:
                self.pop()
            if not self.at_name('for'):
                _syntax('invalid syntax')
            self.pop()
            target, ts, te = self.parse_target()
            self.set_store(target)
            if not self.at_name('in'):
                _syntax('invalid syntax')
            self.pop()
            iterable, vs, ve = self.parse_or()
            filters = []
            while self.at_name('if'):
                self.pop()
                condition, cs, ce = self.parse_or()
                filters.append(condition)
            generators.append(comprehension(target=target, iter=iterable, ifs=filters, is_async=asynchronous))
        return generators

    def parse_atom(self):
        tok = self.toks[self.at]
        kind = tok.kind
        if kind == 'name' and tok.text == 'lambda':
            self.pop()
            params = self.parse_parameters(':')
            self.expect_op(':')
            body, bs, be = self.parse_expr(False)
            node = self.located(Lambda, tok, self.toks[self.at - 1], args=params, body=body)
            return node, (tok.srow, tok.scol), be
        if kind == 'name' and tok.text == 'yield':
            self.pop()
            delegated = self.at_name('from')
            if delegated:
                self.pop()
            value = None
            if self.toks[self.at].kind not in ('end', 'newline') and not self.at_op(')') and not self.at_op(';'):
                value, vs, ve = self.parse_expr(True)
            node = self.located(YieldFrom if delegated else Yield, tok, self.toks[self.at - 1], value=value)
            return node, (tok.srow, tok.scol), (node.end_lineno, node.end_col_offset)

        if kind == 'op' and tok.text == '*':
            self.pop()
            value, vs, ve = self.parse_bitor()
            node = self.located(Starred, tok, self.toks[self.at - 1], value=value, ctx=Load())
            return node, (tok.srow, tok.scol), ve
        if kind == 'name' and tok.text == 'await':
            self.pop()
            value, vs, ve = self.parse_postfix()
            node = self.located(Await, tok, self.toks[self.at - 1], value=value)
            return node, (tok.srow, tok.scol), ve

        if kind == 'num':
            self.pop()
            if '_' in tok.text and self.fv < 6:
                _syntax('Underscores in numeric literals are only supported in Python 3.6 and greater')
            node = Constant(value=_number(tok.text, self.source),
                            lineno=tok.srow, col_offset=tok.scol,
                            end_lineno=tok.erow, end_col_offset=tok.ecol)
            return (node, (tok.srow, tok.scol), (tok.erow, tok.ecol))
        if kind == 'name':
            self.pop()
            word = tok.text
            if word == 'True':
                return self.keyword_constant(tok, True)
            if word == 'False':
                return self.keyword_constant(tok, False)
            if word == 'None':
                return self.keyword_constant(tok, None)
            if word == '__debug__' and (self.fold_debug or self.optimize >= 1):
                return self.keyword_constant(tok, self.optimize < 1)
            if word in _STMT_WORDS or word in ('and', 'or', 'is', 'in', 'not', 'if', 'else'):
                if self.fv < 5 and word in ('async', 'await'):
                    pass
                elif word == 'await':
                    _syntax('invalid syntax')
                else:
                    _no_tree()
            node = Name(id=word, ctx=Load(), lineno=tok.srow,
                        col_offset=tok.scol, end_lineno=tok.erow,
                        end_col_offset=tok.ecol)
            return (node, (tok.srow, tok.scol), (tok.erow, tok.ecol))
        if kind in ('str', 'fstr', 'tstr'):
            return self.parse_string()
        if kind == 'op' and tok.text == '...':
            self.pop()
            node = Constant(value=Ellipsis, lineno=tok.srow,
                            col_offset=tok.scol, end_lineno=tok.erow,
                            end_col_offset=tok.ecol)
            return (node, (tok.srow, tok.scol), (tok.erow, tok.ecol))
        if kind == 'op' and tok.text == '(':
            open_tok = self.pop()
            if self.at_op(')'):
                close_tok = self.pop()
                node = Tuple(elts=[], ctx=Load(), lineno=open_tok.srow,
                             col_offset=open_tok.scol,
                             end_lineno=close_tok.erow,
                             end_col_offset=close_tok.ecol)
                return (node, (open_tok.srow, open_tok.scol),
                        (close_tok.erow, close_tok.ecol))
            node, ls, le = self.parse_expr(False)
            if self.at_name('for') or self.at_name('async'):
                generators = self.parse_generators()
                close = self.expect_op(')')
                node = self.located(GeneratorExp, open_tok, close, elt=node, generators=generators)
                return node, (open_tok.srow, open_tok.scol), (close.erow, close.ecol)
            if self.at_op(','):
                elts = [node]
                while self.at_op(','):
                    self.pop()
                    if self.at_op(')'):
                        break
                    elt, es, ee = self.parse_expr(False)
                    elts.append(elt)
                close_tok = self.expect_op(')')
                node = Tuple(elts=elts, ctx=Load(), lineno=open_tok.srow,
                             col_offset=open_tok.scol,
                             end_lineno=close_tok.erow,
                             end_col_offset=close_tok.ecol)
                return (node, (open_tok.srow, open_tok.scol),
                        (close_tok.erow, close_tok.ecol))
            close_tok = self.expect_op(')')
            return (node, (open_tok.srow, open_tok.scol),
                    (close_tok.erow, close_tok.ecol))
        if kind == 'op' and tok.text == '[':
            open_tok = self.pop()
            elts = []
            if not self.at_op(']'):
                while True:
                    elt, es, ee = self.parse_expr(False)
                    if self.at_name('for') or self.at_name('async'):
                        if elts:
                            _syntax('invalid syntax')
                        generators = self.parse_generators()
                        close = self.expect_op(']')
                        node = self.located(ListComp, open_tok, close, elt=elt, generators=generators)
                        return node, (open_tok.srow, open_tok.scol), (close.erow, close.ecol)
                    elts.append(elt)
                    if self.at_op(','):
                        self.pop()
                        if self.at_op(']'):
                            break
                        continue
                    break
            close_tok = self.expect_op(']')
            node = List(elts=elts, ctx=Load(), lineno=open_tok.srow,
                        col_offset=open_tok.scol,
                        end_lineno=close_tok.erow,
                        end_col_offset=close_tok.ecol)
            return (node, (open_tok.srow, open_tok.scol),
                    (close_tok.erow, close_tok.ecol))
        if kind == 'op' and tok.text == '{':
            open_tok = self.pop()
            if self.at_op('}'):
                close_tok = self.pop()
                node = Dict(keys=[], values=[], lineno=open_tok.srow,
                            col_offset=open_tok.scol,
                            end_lineno=close_tok.erow,
                            end_col_offset=close_tok.ecol)
                return (node, (open_tok.srow, open_tok.scol),
                        (close_tok.erow, close_tok.ecol))
            keys = []
            values = []
            elements = []
            dictionary = None
            while True:
                if self.at_op('**'):
                    self.pop()
                    key = None
                    value, vs, ve = self.parse_expr(False)
                    if dictionary is False:
                        _syntax('invalid syntax')
                    dictionary = True
                else:
                    key, ks, ke = self.parse_expr(False)
                    if self.at_op(':'):
                        self.pop()
                        value, vs, ve = self.parse_expr(False)
                        if dictionary is False:
                            _syntax('invalid syntax')
                        dictionary = True
                    else:
                        if dictionary:
                            _syntax("':' expected after dictionary key")
                        dictionary = False
                        value = key
                if self.at_name('for') or self.at_name('async'):
                    generators = self.parse_generators()
                    close = self.expect_op('}')
                    if dictionary:
                        node = self.located(DictComp, open_tok, close, key=key, value=value, generators=generators)
                    else:
                        node = self.located(SetComp, open_tok, close, elt=value, generators=generators)
                    return node, (open_tok.srow, open_tok.scol), (close.erow, close.ecol)
                if dictionary:
                    keys.append(key)
                    values.append(value)
                else:
                    elements.append(value)
                if not self.at_op(','):
                    break
                self.pop()
                if self.at_op('}'):
                    break
            close = self.expect_op('}')
            if dictionary:
                node = self.located(Dict, open_tok, close, keys=keys, values=values)
            else:
                node = self.located(Set, open_tok, close, elts=elements)
            return node, (open_tok.srow, open_tok.scol), (close.erow, close.ecol)
        _syntax('invalid syntax')

    def keyword_constant(self, tok, value):
        node = Constant(value=value, lineno=tok.srow, col_offset=tok.scol,
                        end_lineno=tok.erow, end_col_offset=tok.ecol)
        return (node, (tok.srow, tok.scol), (tok.erow, tok.ecol))

    def parse_string(self):
        # One string atom: a literal, or several implicitly concatenated,
        # some of them f-strings. A plain literal in a joined string keeps
        # its whole literal span; an f-string text piece keeps exactly its
        # source, and two text pieces side by side after concatenation
        # merge, keeping the first's start and the last's end.
        toks = [self.pop()]
        while self.toks[self.at].kind in ('str', 'fstr', 'tstr'):
            toks.append(self.pop())
        first = toks[0]
        last = toks[-1]
        has_f = False
        for tok in toks:
            if tok.kind in ('fstr', 'tstr'):
                has_f = True
        if not has_f:
            value = first.value
            for tok in toks[1:]:
                if type(value) != type(tok.value):
                    _syntax('cannot mix bytes and nonbytes literals')
                value = value + tok.value
            kind = None
            if 'u' in first.prefix:
                kind = 'u'
            node = Constant(value=value, kind=kind, lineno=first.srow,
                            col_offset=first.scol, end_lineno=last.erow,
                            end_col_offset=last.ecol)
            return (node, (first.srow, first.scol), (last.erow, last.ecol))
        pieces = []
        for tok in toks:
            if tok.kind == 'str':
                kind = None
                if 'u' in tok.prefix:
                    kind = 'u'
                pieces.append(Constant(value=tok.value, kind=kind,
                                       lineno=tok.srow, col_offset=tok.scol,
                                       end_lineno=tok.erow,
                                       end_col_offset=tok.ecol))
            else:
                raw = 'r' in tok.prefix
                values = _joined_values(self.source, tok.cs, tok.ce,
                                        tok.cs_row, tok.cs_col, raw, False,
                                        len(tok.quote) == 3, tok.kind == 'tstr')
                for value in values:
                    pieces.append(value)
        values = []
        for piece in pieces:
            if values and type(values[-1]) == Constant and type(piece) == Constant:
                prev = values[-1]
                values[-1] = Constant(value=prev.value + piece.value,
                                      kind=prev.kind,
                                      lineno=prev.lineno,
                                      col_offset=prev.col_offset,
                                      end_lineno=piece.end_lineno,
                                      end_col_offset=piece.end_col_offset)
            else:
                values.append(piece)
        node = (TemplateStr if first.kind == 'tstr' else JoinedStr)(values=values, lineno=first.srow,
                         col_offset=first.scol, end_lineno=last.erow,
                         end_col_offset=last.ecol)
        return (node, (first.srow, first.scol), (last.erow, last.ecol))

def _type_comment_payload(text):
    # The payload of a type comment, or None when the comment is not one.
    payload = _comment_payload(text)
    if payload is None or _is_ignore(payload):
        return None
    return payload

def _check_type_comments(toks, comments):
    # One pass over tokens and comments in source order. Inside a def's
    # parameter list, a type comment that follows the comma closing a
    # lone '*' parameter -- on the same line or a later one -- is
    # attached to that bare star, and CPython rejects it. Comments
    # deeper in the brackets or outside a parameter list attach to
    # constructs this reader does not check.
    tlen = len(toks) - 1
    clen = len(comments)
    ti = 0
    ci = 0
    depth = 0
    base = -1
    seg = []
    closed_star = False
    after_comma = False
    while ti < tlen or ci < clen:
        if ci < clen and (ti >= tlen or
                          (comments[ci][0], comments[ci][1]) <
                          (toks[ti].srow, toks[ti].scol)):
            if (base >= 0 and depth == base and after_comma and
                    closed_star and
                    _type_comment_payload(comments[ci][2]) is not None):
                _syntax('bare * has associated type comment')
            ci += 1
            continue
        tok = toks[ti]
        ti += 1
        if tok.kind == 'op' and tok.text in ('(', '[', '{'):
            if (tok.text == '(' and base < 0 and ti >= 3 and
                    toks[ti - 3].kind == 'name' and toks[ti - 3].text == 'def' and
                    toks[ti - 2].kind == 'name'):
                base = depth + 1
                seg = []
                closed_star = False
                after_comma = False
            depth += 1
            continue
        if tok.kind == 'op' and tok.text in (')', ']', '}'):
            depth -= 1
            if base >= 0 and depth < base:
                base = -1
            continue
        if base < 0 or depth != base:
            continue
        if tok.kind == 'op' and tok.text == ',':
            closed_star = len(seg) == 1 and seg[0] == '*'
            seg = []
            after_comma = True
            continue
        seg.append(tok.text)
        after_comma = False

def _compound_statement(node):
    return isinstance(node, (FunctionDef, AsyncFunctionDef, ClassDef, If, While,
                            For, AsyncFor, With, AsyncWith, Try, TryStar, Match))

def _parse_for_compile(source, filename, mode, type_comments, optimize):
    # compile() with PyCF_ONLY_AST hands the text here with the flags
    # already read: whether type comments are wanted, and the
    # optimisation setting, both positional here because the kernels'
    # own call carries no keywords.
    return parse(source, filename, mode, type_comments=bool(type_comments),
                 optimize=optimize)

def parse(source, filename='<unknown>', mode='exec', *, type_comments=False,
          feature_version=None, optimize=-1, fold_debug=False):
    if isinstance(source, (bytes, bytearray)):
        # Source bytes name their own encoding: a byte-order mark or a coding comment.
        import io, tokenize
        source = bytes(source)
        source = source.decode(tokenize.detect_encoding(io.BytesIO(source).readline)[0])
    if not isinstance(source, str):
        raise TypeError('compile() arg 1 must be a string, bytes or AST object')
    if '\x00' in source:
        raise SyntaxError('source code string cannot contain null bytes')
    if optimize not in (-1, 0, 1, 2):
        raise ValueError('compile(): invalid optimize value')
    source = source.replace('\r\n', '\n').replace('\r', '\n')
    if mode == 'func_type':
        toks = _lex(source, 0, len(source), 1, 0, False)
        parser = _Parser(source, toks, optimize, fold_debug, fv=feature_version)
        tree = parser.parse_func_type()
        _check_features(tree, feature_version)
        return tree
    if mode == 'eval':
        toks = _lex(source, 0, len(source), 1, 0, False)
        parser = _Parser(source, toks, optimize, fold_debug, fv=feature_version)
        while parser.peek().kind == 'newline':
            parser.pop()
        if parser.peek().scol > 0:
            raise IndentationError('unexpected indent')
        node, ls, le = parser.parse_expr(True)
        while parser.peek().kind == 'newline':
            parser.pop()
        if parser.peek().kind != 'end':
            _syntax('invalid syntax')
        tree = Expression(body=node)
        _check_features(tree, feature_version)
        return tree
    if mode not in ('exec', 'single'):
        raise ValueError("compile() mode must be 'exec', 'eval', 'single' or 'func_type'")
    comments = []
    toks = _lex(source, 0, len(source), 1, 0, False, comments)
    if type_comments:
        _check_type_comments(toks, comments)
    parser = _Parser(source, toks, optimize, fold_debug, comments=comments if type_comments else None, fv=feature_version)
    tree = parser.parse_module(single=mode == 'single')
    tree._lumen_tree_source = source
    if type_comments:
        parser.check_comments()
        for row, column, text in comments:
            tag = _type_ignore_tag(text)
            if tag is not None:
                tree.type_ignores.append(TypeIgnore(lineno=row, tag=tag))
    if mode == 'single':
        if not tree.body:
            raise SyntaxError('invalid syntax')
        tree = Interactive(body=tree.body)
    _check_features(tree, feature_version)
    return tree

def _tree_for_compile(source, filename, mode, type_comments, optimize):
    # compile() reaches the reader with the flags already read: whether
    # type comments were asked for, and the optimisation those flags
    # imply, both positional because the kernels' own call carries no
    # keywords. A level below nought is PyCF_ONLY_AST alone, whose tree
    # leaves the names the optimiser would settle as they are; a level
    # from nought up is PyCF_OPTIMIZED_AST, whose tree settles them.
    if optimize < 0:
        return parse(source, filename, mode, type_comments=bool(type_comments))
    return parse(source, filename, mode, type_comments=bool(type_comments),
                 optimize=optimize, fold_debug=True)

def dump(
    node, annotate_fields=True, include_attributes=False,
    *,
    indent=None, show_empty=False,
):
    """
    Return a formatted dump of the tree in node.  This is mainly useful for
    debugging purposes.  If annotate_fields is true (by default),
    the returned string will show the names and the values for fields.
    If annotate_fields is false, the result string will be more compact by
    omitting unambiguous field names.  Attributes such as line
    numbers and column offsets are not dumped by default.  If this is wanted,
    include_attributes can be set to true.  If indent is a non-negative
    integer or string, then the tree will be pretty-printed with that indent
    level. None (the default) selects the single line representation.
    If show_empty is False, then empty lists and fields that are None
    will be omitted from the output for better readability.
    """
    def _format(node, level=0):
        if indent is not None:
            level += 1
            prefix = '\n' + indent * level
            sep = ',\n' + indent * level
        else:
            prefix = ''
            sep = ', '
        if isinstance(node, AST):
            cls = type(node)
            args = []
            args_buffer = []
            allsimple = True
            keywords = annotate_fields
            for name in node._fields:
                try:
                    value = getattr(node, name)
                except AttributeError:
                    keywords = True
                    continue
                if value is None and getattr(cls, name, ...) is None:
                    keywords = True
                    continue
                if not show_empty:
                    if value == []:
                        field_type = cls._field_types.get(name, object)
                        if getattr(field_type, '__origin__', ...) is list:
                            if not keywords:
                                args_buffer.append(repr(value))
                            continue
                    if not keywords:
                        args.extend(args_buffer)
                        args_buffer = []
                value, simple = _format(value, level)
                allsimple = allsimple and simple
                if keywords:
                    args.append('%s=%s' % (name, value))
                else:
                    args.append(value)
            if include_attributes and node._attributes:
                for name in node._attributes:
                    try:
                        value = getattr(node, name)
                    except AttributeError:
                        continue
                    if value is None and getattr(cls, name, ...) is None:
                        continue
                    value, simple = _format(value, level)
                    allsimple = allsimple and simple
                    args.append('%s=%s' % (name, value))
            if allsimple and len(args) <= 3:
                return '%s(%s)' % (node.__class__.__name__, ', '.join(args)), not args
            return '%s(%s%s)' % (node.__class__.__name__, prefix, sep.join(args)), False
        elif isinstance(node, list):
            if not node:
                return '[]', True
            return '[%s%s]' % (prefix, sep.join(_format(x, level)[0] for x in node)), False
        return repr(node), True

    if not isinstance(node, AST):
        raise TypeError('expected AST, got %r' % node.__class__.__name__)
    if indent is not None and not isinstance(indent, str):
        indent = ' ' * indent
    return _format(node)[0]


def copy_location(new_node, old_node):
    """
    Copy source location (`lineno`, `col_offset`, `end_lineno`, and `end_col_offset`
    attributes) from *old_node* to *new_node* if possible, and return *new_node*.
    """
    for attr in 'lineno', 'col_offset', 'end_lineno', 'end_col_offset':
        if attr in old_node._attributes and attr in new_node._attributes:
            value = getattr(old_node, attr, None)
            # end_lineno and end_col_offset are optional attributes, and they
            # should be copied whether the value is None or not.
            if value is not None or (
                hasattr(old_node, attr) and attr.startswith("end_")
            ):
                setattr(new_node, attr, value)
    return new_node


def fix_missing_locations(node):
    """
    When you compile a node tree with compile(), the compiler expects lineno and
    col_offset attributes for every node that supports them.  This is rather
    tedious to fill in for generated nodes, so this helper adds these attributes
    recursively where not already set, by setting them to the values of the
    parent node.  It works recursively starting at *node*.
    """
    def _fix(node, lineno, col_offset, end_lineno, end_col_offset):
        if 'lineno' in node._attributes:
            if not hasattr(node, 'lineno'):
                node.lineno = lineno
            else:
                lineno = node.lineno
        if 'end_lineno' in node._attributes:
            if getattr(node, 'end_lineno', None) is None:
                node.end_lineno = end_lineno
            else:
                end_lineno = node.end_lineno
        if 'col_offset' in node._attributes:
            if not hasattr(node, 'col_offset'):
                node.col_offset = col_offset
            else:
                col_offset = node.col_offset
        if 'end_col_offset' in node._attributes:
            if getattr(node, 'end_col_offset', None) is None:
                node.end_col_offset = end_col_offset
            else:
                end_col_offset = node.end_col_offset
        for child in iter_child_nodes(node):
            _fix(child, lineno, col_offset, end_lineno, end_col_offset)
    _fix(node, 1, 0, 1, 0)
    return node


def increment_lineno(node, n=1):
    """
    Increment the line number and end line number of each node in the tree
    starting at *node* by *n*. This is useful to "move code" to a different
    location in a file.
    """
    for child in walk(node):
        # TypeIgnore is a special case where lineno is not an attribute
        # but rather a field of the node itself.
        if isinstance(child, TypeIgnore):
            child.lineno = getattr(child, 'lineno', 0) + n
            continue

        if 'lineno' in child._attributes:
            child.lineno = getattr(child, 'lineno', 0) + n
        if (
            "end_lineno" in child._attributes
            and (end_lineno := getattr(child, "end_lineno", 0)) is not None
        ):
            child.end_lineno = end_lineno + n
    return node


def iter_fields(node):
    """
    Yield a tuple of ``(fieldname, value)`` for each field in ``node._fields``
    that is present on *node*.
    """
    for field in node._fields:
        try:
            yield field, getattr(node, field)
        except AttributeError:
            pass


def iter_child_nodes(node):
    """
    Yield all direct child nodes of *node*, that is, all fields that are nodes
    and all items of fields that are lists of nodes.
    """
    for name, field in iter_fields(node):
        if isinstance(field, AST):
            yield field
        elif isinstance(field, list):
            for item in field:
                if isinstance(item, AST):
                    yield item


def get_docstring(node, clean=True):
    """
    Return the docstring for the given node or None if no docstring can
    be found.  If the node provided does not have docstrings a TypeError
    will be raised.

    If *clean* is `True`, all tabs are expanded to spaces and any whitespace
    that can be uniformly removed from the second line onwards is removed.
    """
    if not isinstance(node, (AsyncFunctionDef, FunctionDef, ClassDef, Module)):
        raise TypeError("%r can't have docstrings" % node.__class__.__name__)
    if not(node.body and isinstance(node.body[0], Expr)):
        return None
    node = node.body[0].value
    if isinstance(node, Constant) and isinstance(node.value, str):
        text = node.value
    else:
        return None
    if clean:
        import inspect
        text = inspect.cleandoc(text)
    return text


_line_pattern = None
def _splitlines_no_ff(source, maxlines=None):
    """Split a string into lines ignoring form feed and other chars.

    This mimics how the Python parser splits source code.
    """
    global _line_pattern
    if _line_pattern is None:
        # lazily computed to speedup import time of `ast`
        import re
        _line_pattern = re.compile(r"(.*?(?:\r\n|\n|\r|$))")

    lines = []
    for lineno, match in enumerate(_line_pattern.finditer(source), 1):
        if maxlines is not None and lineno > maxlines:
            break
        lines.append(match[0])
    return lines


def _pad_whitespace(source):
    r"""Replace all chars except '\f\t' in a line with spaces."""
    result = ''
    for c in source:
        if c in '\f\t':
            result += c
        else:
            result += ' '
    return result


def get_source_segment(source, node, *, padded=False):
    """Get source code segment of the *source* that generated *node*.

    If some location information (`lineno`, `end_lineno`, `col_offset`,
    or `end_col_offset`) is missing, return None.

    If *padded* is `True`, the first line of a multi-line statement will
    be padded with spaces to match its original position.
    """
    try:
        if node.end_lineno is None or node.end_col_offset is None:
            return None
        lineno = node.lineno - 1
        end_lineno = node.end_lineno - 1
        col_offset = node.col_offset
        end_col_offset = node.end_col_offset
    except AttributeError:
        return None

    lines = _splitlines_no_ff(source, maxlines=end_lineno+1)
    if end_lineno == lineno:
        return lines[lineno].encode()[col_offset:end_col_offset].decode()

    if padded:
        padding = _pad_whitespace(lines[lineno].encode()[:col_offset].decode())
    else:
        padding = ''

    first = padding + lines[lineno].encode()[col_offset:].decode()
    last = lines[end_lineno].encode()[:end_col_offset].decode()
    lines = lines[lineno+1:end_lineno]

    lines.insert(0, first)
    lines.append(last)
    return ''.join(lines)


def walk(node):
    """
    Recursively yield all descendant nodes in the tree starting at *node*
    (including *node* itself), in no specified order.  This is useful if you
    only want to modify nodes in place and don't care about the context.
    """
    todo = [node]
    index = 0
    while index < len(todo):
        node = todo[index]
        index += 1
        todo.extend(iter_child_nodes(node))
        yield node


def compare(
    a,
    b,
    /,
    *,
    compare_attributes=False,
):
    """Recursively compares two ASTs.

    compare_attributes affects whether AST attributes are considered
    in the comparison. If compare_attributes is False (default), then
    attributes are ignored. Otherwise they must all be equal. This
    option is useful to check whether the ASTs are structurally equal but
    might differ in whitespace or similar details.
    """

    sentinel = object()  # handle the possibility of a missing attribute/field

    def _compare(a, b):
        # Compare two fields on an AST object, which may themselves be
        # AST objects, lists of AST objects, or primitive ASDL types
        # like identifiers and constants.
        if isinstance(a, AST):
            return compare(
                a,
                b,
                compare_attributes=compare_attributes,
            )
        elif isinstance(a, list):
            # If a field is repeated, then both objects will represent
            # the value as a list.
            if len(a) != len(b):
                return False
            for a_item, b_item in zip(a, b):
                if not _compare(a_item, b_item):
                    return False
            else:
                return True
        else:
            return type(a) is type(b) and a == b

    def _compare_fields(a, b):
        if a._fields != b._fields:
            return False
        for field in a._fields:
            a_field = getattr(a, field, sentinel)
            b_field = getattr(b, field, sentinel)
            if a_field is sentinel and b_field is sentinel:
                # both nodes are missing a field at runtime
                continue
            if a_field is sentinel or b_field is sentinel:
                # one of the node is missing a field
                return False
            if not _compare(a_field, b_field):
                return False
        else:
            return True

    def _compare_attributes(a, b):
        if a._attributes != b._attributes:
            return False
        # Attributes are always ints.
        for attr in a._attributes:
            a_attr = getattr(a, attr, sentinel)
            b_attr = getattr(b, attr, sentinel)
            if a_attr is sentinel and b_attr is sentinel:
                # both nodes are missing an attribute at runtime
                continue
            if a_attr != b_attr:
                return False
        else:
            return True

    if type(a) is not type(b):
        return False
    if not _compare_fields(a, b):
        return False
    if compare_attributes and not _compare_attributes(a, b):
        return False
    return True


class NodeVisitor(object):
    """
    A node visitor base class that walks the abstract syntax tree and calls a
    visitor function for every node found.  This function may return a value
    which is forwarded by the `visit` method.

    This class is meant to be subclassed, with the subclass adding visitor
    methods.

    Per default the visitor functions for the nodes are ``'visit_'`` +
    class name of the node.  So a `TryFinally` node visit function would
    be `visit_TryFinally`.  This behavior can be changed by overriding
    the `visit` method.  If no visitor function exists for a node
    (return value `None`) the `generic_visit` visitor is used instead.

    Don't use the `NodeVisitor` if you want to apply changes to nodes during
    traversing.  For this a special visitor exists (`NodeTransformer`) that
    allows modifications.
    """

    def visit(self, node):
        """Visit a node."""
        method = 'visit_' + node.__class__.__name__
        visitor = getattr(self, method, self.generic_visit)
        return visitor(node)

    def generic_visit(self, node):
        """Called if no explicit visitor function exists for a node."""
        for field, value in iter_fields(node):
            if isinstance(value, list):
                for item in value:
                    if isinstance(item, AST):
                        self.visit(item)
            elif isinstance(value, AST):
                self.visit(value)


class NodeTransformer(NodeVisitor):
    """
    A :class:`NodeVisitor` subclass that walks the abstract syntax tree and
    allows modification of nodes.

    The `NodeTransformer` will walk the AST and use the return value of the
    visitor methods to replace or remove the old node.  If the return value of
    the visitor method is ``None``, the node will be removed from its location,
    otherwise it is replaced with the return value.  The return value may be the
    original node in which case no replacement takes place.

    Here is an example transformer that rewrites all occurrences of name lookups
    (``foo``) to ``data['foo']``::

       class RewriteName(NodeTransformer):

           def visit_Name(self, node):
               return Subscript(
                   value=Name(id='data', ctx=Load()),
                   slice=Constant(value=node.id),
                   ctx=node.ctx
               )

    Keep in mind that if the node you're operating on has child nodes you must
    either transform the child nodes yourself or call the :meth:`generic_visit`
    method for the node first.

    For nodes that were part of a collection of statements (that applies to all
    statement nodes), the visitor may also return a list of nodes rather than
    just a single node.

    Usually you use the transformer like this::

       node = YourTransformer().visit(node)
    """

    def generic_visit(self, node):
        for field, old_value in iter_fields(node):
            if isinstance(old_value, list):
                new_values = []
                for value in old_value:
                    if isinstance(value, AST):
                        value = self.visit(value)
                        if value is None:
                            continue
                        elif not isinstance(value, AST):
                            new_values.extend(value)
                            continue
                    new_values.append(value)
                old_value[:] = new_values
            elif isinstance(old_value, AST):
                new_node = self.visit(old_value)
                if new_node is None:
                    delattr(node, field)
                else:
                    setattr(node, field, new_node)
        return node

# Generate source from the current tree, including edits by visitors.
def unparse(ast_obj):
    from _ast_unparse import Unparser
    return Unparser().visit(ast_obj)

# Convert literal trees using the same validation as the standard library.
def _literal_node(node_or_string):
    if isinstance(node_or_string, Expression):
        node_or_string = node_or_string.body
    def _raise_malformed_node(node):
        msg = "malformed node or string"
        if lno := getattr(node, 'lineno', None):
            msg += f' on line {lno}'
        raise ValueError(msg + f': {node!r}')
    def _convert_num(node):
        if not isinstance(node, Constant) or type(node.value) not in (int, float, complex):
            _raise_malformed_node(node)
        return node.value
    def _convert_signed_num(node):
        if isinstance(node, UnaryOp) and isinstance(node.op, (UAdd, USub)):
            operand = _convert_num(node.operand)
            if isinstance(node.op, UAdd):
                return + operand
            else:
                return - operand
        return _convert_num(node)
    def _convert(node):
        if isinstance(node, Constant):
            return node.value
        elif isinstance(node, Tuple):
            return tuple(map(_convert, node.elts))
        elif isinstance(node, List):
            return list(map(_convert, node.elts))
        elif isinstance(node, Set):
            return set(map(_convert, node.elts))
        elif (isinstance(node, Call) and isinstance(node.func, Name) and
              node.func.id == 'set' and node.args == node.keywords == []):
            return set()
        elif isinstance(node, Dict):
            if len(node.keys) != len(node.values):
                _raise_malformed_node(node)
            return dict(zip(map(_convert, node.keys),
                            map(_convert, node.values)))
        elif isinstance(node, BinOp) and isinstance(node.op, (Add, Sub)):
            left = _convert_signed_num(node.left)
            right = _convert_num(node.right)
            if isinstance(left, (int, float)) and isinstance(right, complex):
                if isinstance(node.op, Add):
                    return left + right
                else:
                    return left - right
        return _convert_signed_num(node)
    return _convert(node_or_string)

# Enforce older grammar versions on the resulting syntax tree.
def _check_features(tree, version):
    if version is None:
        return
    if isinstance(version, tuple):
        major, version = version
        if major != 3:
            raise ValueError('Unsupported major version: ' + str(major))
    if not isinstance(version, int):
        raise TypeError("'" + type(version).__name__ + "' object cannot be interpreted as an integer")
    if version < 0:
        return
    for node in walk(tree):
        minimum = 0
        if isinstance(node, NamedExpr):
            minimum = 8
        elif isinstance(node, arguments) and node.posonlyargs:
            minimum = 8
        elif isinstance(node, Match):
            minimum = 10
        elif isinstance(node, TryStar):
            minimum = 11
        elif isinstance(node, (TypeAlias, type_param)) or getattr(node, 'type_params', []):
            minimum = 12
        elif isinstance(node, TemplateStr):
            minimum = 14
        elif isinstance(node, (AsyncFunctionDef, AsyncFor, AsyncWith, Await, MatMult)):
            minimum = 5
        elif isinstance(node, comprehension) and node.is_async:
            minimum = 6
        if isinstance(node, type_param) and node.default_value is not None:
            minimum = 13
        if version < minimum:
            raise SyntaxError(type(node).__name__ + ' is only supported in Python 3.' + str(minimum) + ' and greater')

# Preserve trees while accepting source through the existing buffer protocol.
def _compile_input(source):
    if isinstance(source, (AST, str, bytes)):
        return source
    import builtins
    return builtins._buffer_bytes(source)

# Route compile's AST-only flags through the same public parser.
def _compile_ast(source, filename, mode, flags, optimize=-1, feature_version=None):
    if flags < 0 or flags & 255:
        raise ValueError('compile(): unrecognised flags')
    if optimize not in (-1, 0, 1, 2):
        raise ValueError('compile(): invalid optimize value')
    if mode not in ('exec', 'eval', 'single', 'func_type'):
        raise ValueError('compile(): invalid mode')
    if isinstance(source, AST):
        expected = {'exec': Module, 'eval': Expression, 'single': Interactive, 'func_type': FunctionType}[mode]
        if not isinstance(source, expected):
            raise TypeError('expected ' + expected.__name__ + ' node, got ' + type(source).__name__)
        _validate_node(source)
        _validate_contexts(source)
        source = _copy_ast(source)
        return _optimized_tree(source, max(optimize, 0), True) if flags & 32768 else source
    return parse(source, filename, mode, type_comments=bool(flags & PyCF_TYPE_COMMENTS),
                 optimize=optimize if flags & 32768 else -1, feature_version=feature_version,
                 fold_debug=bool(flags & 32768))

# Validate required fields, locations and ASDL categories before generating code.
def _validate_node(node, active=None):
    if active is None:
        active = set()
    identity = id(node)
    if identity in active:
        raise RecursionError('maximum recursion depth exceeded during compilation')
    active.add(identity)
    if not isinstance(node, AST):
        raise TypeError('expected AST, got ' + repr(type(node).__name__))
    cls = type(node)
    category = 'stmt' if isinstance(node, stmt) else 'expr' if isinstance(node, expr) else cls.__name__
    for attr in ('lineno', 'col_offset'):
        if attr in node._attributes:
            if not hasattr(node, attr):
                raise TypeError('required field "' + attr + '" missing from ' + category)
            if not isinstance(getattr(node, attr), int):
                raise ValueError('invalid integer value: ' + str(getattr(node, attr)))
    for attr in ('end_lineno', 'end_col_offset'):
        if attr in node._attributes and getattr(node, attr, None) is not None and not isinstance(getattr(node, attr), int):
            raise ValueError('invalid integer value: ' + str(getattr(node, attr)))
    if getattr(node, 'end_lineno', None) is not None and node.end_lineno < node.lineno:
        raise ValueError('AST node line range (' + str(node.lineno) + ', ' + str(node.end_lineno) + ') is not valid')
    if getattr(node, 'end_col_offset', None) is not None and node.end_lineno == node.lineno and node.end_col_offset < node.col_offset:
        raise ValueError('line ' + str(node.lineno) + ', column ' + str(node.col_offset) + '-' + str(node.end_col_offset) + ' is not a valid range')
    for field in node._fields:
        if not hasattr(node, field):
            raise TypeError('required field "' + field + '" missing from ' + cls.__name__)
        value = getattr(node, field)
        typ = cls._field_types.get(field, object)
        if getattr(typ, '__origin__', None) is list:
            if not isinstance(value, list):
                raise TypeError(cls.__name__ + ' field "' + field + '" must be a list, not a ' + type(value).__name__)
            element_type = typ.__args__[0]
            for item in value:
                if item is None and ((isinstance(node, arguments) and field == 'kw_defaults') or (isinstance(node, Dict) and field == 'keys')):
                    continue
                if not isinstance(item, element_type):
                    raise TypeError('expected some sort of ' + element_type.__name__ + ', but got ' + repr(item))
                if isinstance(item, AST):
                    _validate_node(item, active)
        elif value is None and getattr(cls, field, ...) is None:
            continue
        elif not isinstance(value, typ):
            raise TypeError('expected ' + str(typ) + ', but got ' + repr(value))
        elif isinstance(value, AST):
            _validate_node(value, active)
    # Two shapes the reference refuses once every field is present and
    # well formed: a named expression binds a name, a type alias is
    # named by a name.
    if isinstance(node, NamedExpr) and not isinstance(node.target, Name):
        raise TypeError('NamedExpr target must be a Name')
    if isinstance(node, TypeAlias) and not isinstance(node.name, Name):
        raise TypeError('TypeAlias with non-Name name')
    if isinstance(node, (FunctionDef, AsyncFunctionDef, ClassDef, For, AsyncFor, While, If, With, AsyncWith, ExceptHandler, Try, TryStar)) and not node.body:
        raise ValueError('empty body on ' + cls.__name__)
    if isinstance(node, Assign) and not node.targets:
        raise ValueError('empty targets on Assign')
    if isinstance(node, Delete) and not node.targets:
        raise ValueError('empty targets on Delete')
    if isinstance(node, BoolOp) and len(node.values) < 2:
        raise ValueError('BoolOp with less than 2 values')
    if isinstance(node, Compare):
        if not node.comparators:
            raise ValueError('Compare with no comparators')
        if len(node.ops) != len(node.comparators):
            raise ValueError('Compare has a different number of comparators and operands')
    if isinstance(node, Dict) and len(node.keys) != len(node.values):
        raise ValueError("Dict doesn't have the same number of keys as values")
    if isinstance(node, Constant):
        _validate_constant(node.value)
    active.remove(identity)

# Compile the current tree rather than replaying the text from which it was parsed.
def _compile_source(tree, mode):
    if not isinstance(tree, AST):
        return None
    expected = {'exec': Module, 'eval': Expression, 'single': Interactive}.get(mode)
    if expected is None:
        raise ValueError('compile(): invalid mode')
    if not isinstance(tree, expected):
        raise TypeError('expected ' + expected.__name__ + ' node, got ' + type(tree).__name__)
    _validate_node(tree)
    _validate_contexts(tree)
    source = unparse(tree)
    generated = parse(source, mode='exec' if mode == 'single' else mode)
    if mode == 'single':
        generated = Interactive(body=generated.body)
    positions = []
    _compile_positions(tree, generated, positions, source.split('\n'))
    return source, tuple(positions)

# Match regenerated nodes by their ASDL fields, carrying the current tree's locations.
def _compile_positions(original, generated, positions, lines):
    if isinstance(original, AST) and isinstance(generated, AST):
        if 'lineno' in original._attributes and 'lineno' in generated._attributes:
            # Scanner columns count characters; AST columns count UTF-8 bytes.
            start_col = len(lines[generated.lineno - 1].encode('utf-8')[:generated.col_offset].decode('utf-8'))
            end_col = len(lines[generated.end_lineno - 1].encode('utf-8')[:generated.end_col_offset].decode('utf-8'))
            positions.append((generated.lineno, start_col, generated.end_lineno, end_col) +
                             (original.lineno, original.col_offset,
                              original.end_lineno if original.end_lineno is not None else original.lineno,
                              original.end_col_offset if original.end_col_offset is not None else original.col_offset))
        if type(original) is type(generated):
            for field in original._fields:
                _compile_positions(getattr(original, field), getattr(generated, field), positions, lines)
    elif isinstance(original, list) and isinstance(generated, list):
        for old, new in zip(original, generated):
            _compile_positions(old, new, positions, lines)


# Check target contexts separately so ordinary expression children remain loads.
def _validate_contexts(node, context=Load):
    if isinstance(node, expr) and context is not Load and not isinstance(node, (Name, Attribute, Subscript, Tuple, List, Starred)):
        raise ValueError("expression which can't be assigned to in " + context.__name__ + ' context')
    if hasattr(node, 'ctx') and not isinstance(node.ctx, context):
        raise ValueError('expression must have ' + context.__name__ + ' context but has ' + type(node.ctx).__name__ + ' instead')
    for field, value in iter_fields(node):
        expected = Load
        if field == 'targets' and isinstance(node, (Assign, Delete)):
            expected = Del if isinstance(node, Delete) else Store
        elif field == 'target' and isinstance(node, (AugAssign, AnnAssign, For, AsyncFor, comprehension, NamedExpr)):
            expected = Store
        elif field == 'optional_vars' and isinstance(node, withitem):
            expected = Store
        elif field == 'name' and isinstance(node, TypeAlias):
            expected = Store
        elif field == 'elts' and isinstance(node, (Tuple, List)):
            expected = context
        elif field == 'value' and isinstance(node, Starred):
            expected = context
        if isinstance(value, AST):
            _validate_contexts(value, expected)
        elif isinstance(value, list):
            for child in value:
                if isinstance(child, AST):
                    _validate_contexts(child, expected)

# Reject arbitrary objects hidden inside constant tuples and frozen sets.
def _validate_constant(value):
    if type(value) not in (int, float, complex, bool, str, bytes, tuple, frozenset, type(None), type(Ellipsis)):
        raise TypeError('got an invalid type in Constant: ' + type(value).__name__)
    if type(value) in (tuple, frozenset):
        for item in value:
            _validate_constant(item)


def main(args=None):
    import argparse
    import sys

    parser = argparse.ArgumentParser(color=True)
    parser.add_argument('infile', nargs='?', default='-',
                        help='the file to parse; defaults to stdin')
    parser.add_argument('-m', '--mode', default='exec',
                        choices=('exec', 'single', 'eval', 'func_type'),
                        help='specify what kind of code must be parsed')
    parser.add_argument('--no-type-comments', default=True, action='store_false',
                        help="don't add information about type comments")
    parser.add_argument('-a', '--include-attributes', action='store_true',
                        help='include attributes such as line numbers and '
                             'column offsets')
    parser.add_argument('-i', '--indent', type=int, default=3,
                        help='indentation of nodes (number of spaces)')
    parser.add_argument('--feature-version',
                        type=str, default=None, metavar='VERSION',
                        help='Python version in the format 3.x '
                             '(for example, 3.10)')
    parser.add_argument('-O', '--optimize',
                        type=int, default=-1, metavar='LEVEL',
                        help='optimization level for parser (default -1)')
    parser.add_argument('--show-empty', default=False, action='store_true',
                        help='show empty lists and fields in dump output')
    args = parser.parse_args(args)

    if args.infile == '-':
        name = '<stdin>'
        source = sys.stdin.buffer.read()
    else:
        name = args.infile
        with open(args.infile, 'rb') as infile:
            source = infile.read()

    # Process feature_version
    feature_version = None
    if args.feature_version:
        try:
            major, minor = map(int, args.feature_version.split('.', 1))
        except ValueError:
            parser.error('Invalid format for --feature-version; '
                         'expected format 3.x (for example, 3.10)')

        feature_version = (major, minor)

    tree = parse(source, name, args.mode, type_comments=args.no_type_comments,
                 feature_version=feature_version, optimize=args.optimize)
    print(dump(tree, include_attributes=args.include_attributes,
               indent=args.indent, show_empty=args.show_empty))


def _optimized_tree(tree, optimize, fold_debug=False):
    def pattern_value(node):
        if isinstance(node, Constant) and type(node.value) in (int, float, complex):
            return node.value
        if isinstance(node, UnaryOp) and isinstance(node.op, (UAdd, USub)):
            value = pattern_value(node.operand)
            if value is not None:
                return -value if isinstance(node.op, USub) else +value
        if isinstance(node, BinOp) and isinstance(node.op, (Add, Sub)):
            left, right = pattern_value(node.left), pattern_value(node.right)
            if type(left) in (int, float) and type(right) is complex:
                return left + right if isinstance(node.op, Add) else left - right
        return None

    class Optimizer(NodeTransformer):
        def visit_MatchValue(self, node):
            self.generic_visit(node)
            value = pattern_value(node.value) if optimize > 0 else None
            if value is not None:
                node.value = copy_location(Constant(value=value), node.value)
            return node

        def visit_MatchMapping(self, node):
            self.generic_visit(node)
            if optimize > 0:
                for index, key in enumerate(node.keys):
                    value = pattern_value(key)
                    if value is not None:
                        node.keys[index] = copy_location(Constant(value=value), key)
            return node

        def visit_BinOp(self, node):
            self.generic_visit(node)
            if optimize < 1 or not isinstance(node.op, Mod) or not isinstance(node.left, Constant) or type(node.left.value) is not str or not isinstance(node.right, Tuple):
                return node
            text, arguments = node.left.value, node.right.elts
            if any(isinstance(arg, Starred) for arg in arguments):
                return node
            values, literal, cursor, argument = [], '', 0, 0
            while cursor < len(text):
                if text[cursor] != '%':
                    literal += text[cursor]
                    cursor += 1
                    continue
                cursor += 1
                if cursor < len(text) and text[cursor] == '%':
                    literal += '%'
                    cursor += 1
                    continue
                flags = ''
                while cursor < len(text) and text[cursor] in '-+ #0':
                    flags += text[cursor]
                    cursor += 1
                width_start = cursor
                while cursor < len(text) and '0' <= text[cursor] <= '9':
                    cursor += 1
                width = text[width_start:cursor]
                precision = ''
                if cursor < len(text) and text[cursor] == '.':
                    cursor += 1
                    precision_start = cursor
                    while cursor < len(text) and '0' <= text[cursor] <= '9':
                        cursor += 1
                    precision = '.' + (text[precision_start:cursor] or '0')
                if cursor == len(text) or text[cursor] not in 'sar' or argument == len(arguments):
                    return node
                if literal:
                    values.append(copy_location(Constant(value=literal), node.left))
                    literal = ''
                spec = (('<' if '-' in flags else '>') + width if width else '') + precision
                value = FormattedValue(value=arguments[argument], conversion=ord(text[cursor]),
                                       format_spec=copy_location(Constant(value=spec), node.left) if spec else None)
                values.append(copy_location(value, node))
                argument += 1
                cursor += 1
            if argument != len(arguments):
                return node
            if literal:
                values.append(copy_location(Constant(value=literal), node.left))
            return copy_location(JoinedStr(values=values), node)

        def visit_Name(self, node):
            if node.id == '__debug__' and isinstance(node.ctx, Load) and (fold_debug or optimize > 0):
                return copy_location(Constant(value=optimize < 1), node)
            return node

        def generic_visit(self, node):
            result = super().generic_visit(node)
            if optimize == 2 and isinstance(result, (Module, FunctionDef, AsyncFunctionDef, ClassDef)):
                body = result.body
                if body and isinstance(body[0], Expr) and isinstance(body[0].value, Constant) and isinstance(body[0].value.value, str):
                    removed = body.pop(0)
                    if not body:
                        body.append(copy_location(Pass(), removed))
            return result
    return Optimizer().visit(tree)


def _copy_ast(value):
    if isinstance(value, AST):
        fields = {name: _copy_ast(item) for name, item in iter_fields(value)}
        for name in value._attributes:
            if hasattr(value, name):
                fields[name] = getattr(value, name)
        return type(value)(**fields)
    if isinstance(value, list):
        return [_copy_ast(item) for item in value]
    return value
