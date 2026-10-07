# The part of ast that needs no parser. literal_eval is here in full: it
# reads the literal grammar -- numbers, strings, bytes, the three keyword
# constants, Ellipsis, tuples, lists, dicts, sets, a signed number and a
# number plus or minus an imaginary one -- straight from the source text,
# and accepts exactly what CPython's literal_eval accepts.
#
# A small reader gives parse() a deliberately narrow subset of the grammar
# (see below): enough for the f-string tree tests, with CPython's node
# classes and positions. Everything else a syntax tree would serve --
# walking, dumping, unparsing, and source outside the subset -- still says
# it has no tree rather than return something shaped like one.
#
# With type_comments=True one of CPython's type-comment rules is applied:
# a '# type: ...' comment attached to a bare * in a function's parameter
# list is SyntaxError ('bare * has associated type comment'). Type
# comments in the positions CPython accepts (on assignments, 'for' and
# 'with' headers, a def's own line) are read but not attached to any
# node, and placements CPython rejects outright are not checked, since
# the statements carrying them have no tree in this subset.
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

# --------------------------------------------------------------------------
# A deliberately small reader for source text, answering CPython's node
# classes with CPython's positions. It reads assignment and expression
# statements over names, numbers, strings, f-strings, calls, subscripts,
# attribute reads, tuples, lists, dicts, sets and the common operators.
# Positions follow CPython 3.12+ for f-strings: a FormattedValue covers
# its braces, a text Constant covers exactly its source (a doubled brace
# covers both source braces), and adjacent text pieces merge across
# implicit concatenation, keeping the first piece's start and the last
# piece's end. A parsed tree remembers its source text, which is what
# compile() of the tree compiles.
#
# Source inside the subset that is malformed gets SyntaxError; source
# outside the subset still gets the NotImplementedError above.

def _syntax(message):
    raise 'SyntaxError: ' + message

def _no_tree():
    raise _NO_TREE

class AST:
    _fields = ()
    _attributes = ('lineno', 'col_offset', 'end_lineno', 'end_col_offset')

    def __init__(self, *args, **kwargs):
        for name in self._fields:
            setattr(self, name, None)
        for name in self._attributes:
            setattr(self, name, None)
        count = len(args)
        if count > len(self._fields):
            count = len(self._fields)
        for at in range(count):
            setattr(self, self._fields[at], args[at])
        for key, value in kwargs.items():
            setattr(self, key, value)

class Module(AST):
    _fields = ('body', 'type_ignores')
    _attributes = ()

class Expression(AST):
    _fields = ('body',)
    _attributes = ()

class stmt(AST):
    pass

class expr(AST):
    pass

class Expr(stmt):
    _fields = ('value',)

class Assign(stmt):
    _fields = ('targets', 'value', 'type_comment')

class JoinedStr(expr):
    _fields = ('values',)

class FormattedValue(expr):
    _fields = ('value', 'conversion', 'format_spec')

class Constant(expr):
    _fields = ('value', 'kind')

class Name(expr):
    _fields = ('id', 'ctx')

class BinOp(expr):
    _fields = ('left', 'op', 'right')

class UnaryOp(expr):
    _fields = ('op', 'operand')

class BoolOp(expr):
    _fields = ('op', 'values')

class Compare(expr):
    _fields = ('left', 'ops', 'comparators')

class Call(expr):
    _fields = ('func', 'args', 'keywords')

class Attribute(expr):
    _fields = ('value', 'attr', 'ctx')

class Subscript(expr):
    _fields = ('value', 'slice', 'ctx')

class Tuple(expr):
    _fields = ('elts', 'ctx')

class List(expr):
    _fields = ('elts', 'ctx')

class Dict(expr):
    _fields = ('keys', 'values')

class Set(expr):
    _fields = ('elts',)

class Pass(stmt):
    _fields = ()

class Return(stmt):
    _fields = ('value',)

class Assert(stmt):
    _fields = ('test', 'msg')

class If(stmt):
    _fields = ('test', 'body', 'orelse')

class Try(stmt):
    _fields = ('body', 'handlers', 'orelse', 'finalbody')

class ExceptHandler(AST):
    _fields = ('type', 'name', 'body')

class FunctionDef(stmt):
    _fields = ('name', 'args', 'body', 'decorator_list', 'returns',
               'type_comment', 'type_params')

class arguments(AST):
    _fields = ('posonlyargs', 'args', 'vararg', 'kwonlyargs',
               'kw_defaults', 'kwarg', 'defaults')
    _attributes = ()

class arg(AST):
    _fields = ('arg', 'annotation', 'type_comment')
    _attributes = ()

class expr_context(AST):
    _attributes = ()

class Load(expr_context):
    pass

class Store(expr_context):
    pass

class Del(expr_context):
    pass

class operator(AST):
    _attributes = ()

class Add(operator):
    pass

class Sub(operator):
    pass

class Mult(operator):
    pass

class Div(operator):
    pass

class FloorDiv(operator):
    pass

class Mod(operator):
    pass

class Pow(operator):
    pass

class LShift(operator):
    pass

class RShift(operator):
    pass

class BitOr(operator):
    pass

class BitXor(operator):
    pass

class BitAnd(operator):
    pass

class MatMult(operator):
    pass

class unaryop(AST):
    _attributes = ()

class UAdd(unaryop):
    pass

class USub(unaryop):
    pass

class Not(unaryop):
    pass

class Invert(unaryop):
    pass

class boolop(AST):
    _attributes = ()

class And(boolop):
    pass

class Or(boolop):
    pass

class cmpop(AST):
    _attributes = ()

class Eq(cmpop):
    pass

class NotEq(cmpop):
    pass

class Lt(cmpop):
    pass

class LtE(cmpop):
    pass

class Gt(cmpop):
    pass

class GtE(cmpop):
    pass

class Is(cmpop):
    pass

class IsNot(cmpop):
    pass

class In(cmpop):
    pass

class NotIn(cmpop):
    pass

_STRING_PREFIXES = ('r', 'b', 'u', 'f', 'rb', 'br', 'fr', 'rf')

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
    for _ in range(count):
        if source[i] == '\n':
            row += 1
            col = 0
        else:
            col += 1
        i += 1
    return (i, row, col)

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
    while i < n:
        if source[i] == '\\':
            i += 2
            continue
        if source[i:i + len(closer)] == closer:
            return i + len(closer)
        i += 1
    _syntax('unterminated string literal')

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
            j = i
            while j < end and source[j] != '\n':
                j += 1
            if comments is not None:
                comments.append((srow, scol, source[i:j]))
            col += j - i
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
            toks.append(_Tok('num', source[i:j], srow, scol, srow, scol + (j - i)))
            col += j - i
            i = j
            continue
        if c == '_' or c.isalpha():
            j = i
            while j < end and (source[j] == '_' or source[j].isalnum()):
                j += 1
            word = source[i:j]
            lowered = word.lower()
            if j < end and source[j] in '\'"' and lowered in _STRING_PREFIXES:
                if 'f' in lowered:
                    lit_end, quote, content_end = _fstring_end(source, j)
                    ci, crow, ccol = _adv(source, i, (j - i) + len(quote), row, col)
                    i2, erow, ecol = _adv(source, ci, lit_end - ci, crow, ccol)
                    tok = _Tok('fstr', source[i:lit_end], srow, scol, erow, ecol)
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
            toks.append(_Tok('name', word, srow, scol, srow, scol + (j - i)))
            col += j - i
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

def _joined_values(source, seg_i, seg_end, row, col, raw, spec, multi):
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
            fv, i, row, col = _formatted(source, i, row, col, raw, multi)
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

def _formatted(source, i, row, col, raw, multi):
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
                break
            depth -= 1
            i, row, col = _adv(source, i, 1, row, col)
            continue
        if depth == 0 and c == '=' and source[i + 1:i + 2] != '=' and source[i - 1:i] not in ('=', '!', '<', '>', ':'):
            _no_tree()
        if depth == 0 and c == '!' and source[i + 1:i + 2] != '=':
            if expr_end < 0:
                expr_end = i
            conv = ord(source[i + 1])
            i, row, col = _adv(source, i, 2, row, col)
            continue
        if depth == 0 and c == ':':
            if source[i + 1:i + 2] == '=':
                _no_tree()
            if expr_end < 0:
                expr_end = i
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
    node = FormattedValue(value=value, conversion=conv, format_spec=spec,
                          lineno=srow, col_offset=scol,
                          end_lineno=row, end_col_offset=col + 1)
    i, row, col = _adv(source, i, 1, row, col)
    return (node, i, row, col)

def _parse_field_expr(source, s, e, row, col):
    toks = _lex(source, s, e, row, col, True)
    parser = _Parser(source, toks)
    node, ls, le = parser.parse_expr(False)
    if parser.peek().kind != 'end':
        _syntax('invalid syntax')
    return node

_STMT_WORDS = ('def', 'class', 'if', 'elif', 'else', 'for', 'while', 'try',
               'except', 'finally', 'with', 'return', 'yield', 'raise',
               'import', 'from', 'pass', 'break', 'continue', 'global',
               'nonlocal', 'assert', 'del', 'async', 'await', 'match',
               'case', 'lambda')

_BINOPS = {'+': Add, '-': Sub, '*': Mult, '/': Div, '//': FloorDiv,
           '%': Mod, '@': MatMult, '<<': LShift, '>>': RShift,
           '|': BitOr, '^': BitXor, '&': BitAnd, '**': Pow}
_CMPOPS = {'==': Eq, '!=': NotEq, '<': Lt, '<=': LtE, '>': Gt, '>=': GtE}
_UNARY = {'+': UAdd, '-': USub, '~': Invert}

class _Parser:
    def __init__(self, source, toks, optimize=-1, fold_debug=False):
        self.source = source
        self.toks = toks
        self.at = 0
        self.optimize = optimize
        self.fold_debug = fold_debug

    def peek(self):
        return self.toks[self.at]

    def pop(self):
        tok = self.toks[self.at]
        self.at += 1
        return tok

    def at_op(self, text):
        tok = self.peek()
        return tok.kind == 'op' and tok.text == text

    def at_name(self, word):
        tok = self.peek()
        return tok.kind == 'name' and tok.text == word

    def expect_op(self, text):
        if not self.at_op(text):
            _syntax('invalid syntax')
        return self.pop()

    def parse_module(self):
        return Module(body=self.parse_block(0), type_ignores=[])

    def parse_block(self, indent):
        # A block is the run of statements whose first token stands at
        # the block's own column; a token further left belongs to an
        # enclosing block, one further right is an error.
        body = []
        while True:
            while self.peek().kind == 'newline':
                self.pop()
            tok = self.peek()
            if tok.kind == 'end' or tok.scol < indent:
                break
            if tok.scol > indent:
                _syntax('unexpected indent')
            body.append(self.parse_stmt())
            if not isinstance(body[-1], (If, Try, FunctionDef)):
                while self.at_op(';'):
                    self.pop()
                    if self.peek().kind in ('newline', 'end'):
                        break
                    body.append(self.parse_simple_stmt())
        return body

    def parse_stmt(self):
        # A word that opens a block of its own is read here; anything
        # else, the simple statement words included, is left to the
        # simple-statement reader below.
        tok = self.peek()
        if tok.kind == 'name':
            word = tok.text
            if word == 'def':
                return self.parse_def()
            if word == 'if':
                return self.parse_if()
            if word == 'try':
                return self.parse_try()
            if word in ('elif', 'else', 'except', 'finally'):
                _syntax('invalid syntax')
        return self.parse_simple_stmt()

    def parse_simple_stmt(self):
        # An expression, an assignment, or one of the simple statement
        # words. A word that opens a block of its own is refused: an
        # inline suite and the run after a semicolon take simple
        # statements alone.
        tok = self.peek()
        if tok.kind == 'name':
            word = tok.text
            if word == 'return':
                return self.parse_return()
            if word == 'pass':
                self.pop()
                return Pass(lineno=tok.srow, col_offset=tok.scol,
                            end_lineno=tok.erow, end_col_offset=tok.ecol)
            if word == 'assert':
                return self.parse_assert()
            if word in _STMT_WORDS:
                _no_tree()
        node, ls, le = self.parse_expr(True)
        if self.at_op('='):
            pairs = [(node, ls, le)]
            while self.at_op('='):
                self.pop()
                pairs.append(self.parse_expr(True))
            value, vs, ve = pairs[-1]
            targets = []
            for target, ts, te in pairs[:-1]:
                self.set_store(target)
                targets.append(target)
            first, fs, fe = pairs[0]
            return Assign(targets=targets, value=value,
                          lineno=fs[0], col_offset=fs[1],
                          end_lineno=ve[0], end_col_offset=ve[1])
        return Expr(value=node, lineno=ls[0], col_offset=ls[1],
                    end_lineno=le[0], end_col_offset=le[1])

    def expect_name(self, word):
        tok = self.peek()
        if tok.kind != 'name' or tok.text != word:
            _syntax('invalid syntax')
        return self.pop()

    def tail_of(self, body, fallback):
        if body:
            last = body[-1]
            if getattr(last, 'end_lineno', None) is not None:
                return (last.end_lineno, last.end_col_offset)
        return fallback

    def parse_suite(self, parent_col):
        # The ':' is already read. Either the rest of the line is the
        # suite -- simple statements alone -- or the suite is the run
        # of statements one column deeper.
        if self.peek().kind != 'newline':
            body = [self.parse_simple_stmt()]
            while self.at_op(';'):
                self.pop()
                if self.peek().kind in ('newline', 'end'):
                    break
                body.append(self.parse_simple_stmt())
            return body
        while self.peek().kind == 'newline':
            self.pop()
        tok = self.peek()
        if tok.kind == 'end' or tok.scol <= parent_col:
            _syntax('expected an indented block')
        return self.parse_block(tok.scol)

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
        while self.peek().kind == 'newline':
            self.pop()
        tok = self.peek()
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
            while self.peek().kind == 'newline':
                self.pop()
            head = self.peek()
            if self.at_name('except') and head.scol == tok.scol:
                handlers.append(self.parse_except(tok.scol))
                continue
            break
        while self.peek().kind == 'newline':
            self.pop()
        if self.at_name('else') and self.peek().scol == tok.scol:
            self.pop()
            self.expect_op(':')
            orelse = self.parse_suite(tok.scol)
            while self.peek().kind == 'newline':
                self.pop()
        if self.at_name('finally') and self.peek().scol == tok.scol:
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
        if self.peek().kind not in ('newline', 'end') and not self.at_op(';'):
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
                if self.peek().kind == 'name' and self.peek().text not in _STMT_WORDS:
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

    def set_store(self, node):
        if type(node) == Name or type(node) == Attribute or type(node) == Subscript:
            node.ctx = Store()
            return
        if type(node) == Tuple or type(node) == List:
            node.ctx = Store()
            for elt in node.elts:
                self.set_store(elt)
            return
        _syntax('invalid syntax')

    # Every level answers (node, loose start, loose end): the loose span
    # reaches over parentheses the node itself does not keep, which is
    # where statements and binary operations take their ends from.

    def parse_expr(self, bare_tuple):
        node, ls, le = self.parse_or()
        if self.at_name('if'):
            _no_tree()
        if bare_tuple and self.at_op(','):
            elts = [node]
            while self.at_op(','):
                self.pop()
                tok = self.peek()
                if tok.kind in ('newline', 'end') or self.at_op(')') or self.at_op(']') or self.at_op('}'):
                    break
                elt, es, ee = self.parse_or()
                elts.append(elt)
                le = ee
            node = Tuple(elts=elts, ctx=Load(), lineno=ls[0], col_offset=ls[1],
                         end_lineno=le[0], end_col_offset=le[1])
        return (node, ls, le)

    def parse_or(self):
        node, ls, le = self.parse_and()
        while self.at_name('or'):
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
        while self.at_name('and'):
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
        if self.at_name('not'):
            tok = self.pop()
            operand, os, oe = self.parse_not()
            node = UnaryOp(op=Not(), operand=operand, lineno=tok.srow,
                           col_offset=tok.scol, end_lineno=oe[0],
                           end_col_offset=oe[1])
            return (node, (tok.srow, tok.scol), oe)
        return self.parse_comparison()

    def parse_comparison(self):
        node, ls, le = self.parse_arith()
        ops = []
        comparators = []
        while True:
            tok = self.peek()
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
            right, rs, le = self.parse_arith()
            ops.append(op)
            comparators.append(right)
        if not ops:
            return (node, ls, le)
        node = Compare(left=node, ops=ops, comparators=comparators,
                       lineno=ls[0], col_offset=ls[1],
                       end_lineno=le[0], end_col_offset=le[1])
        return (node, ls, le)

    def parse_arith(self):
        node, ls, le = self.parse_term()
        while self.at_op('+') or self.at_op('-'):
            tok = self.pop()
            right, rs, re = self.parse_term()
            node = BinOp(left=node, op=_BINOPS[tok.text](), right=right,
                         lineno=ls[0], col_offset=ls[1],
                         end_lineno=re[0], end_col_offset=re[1])
            le = re
        return (node, ls, le)

    def parse_term(self):
        node, ls, le = self.parse_unary()
        while True:
            tok = self.peek()
            if tok.kind == 'op' and tok.text in ('*', '/', '//', '%', '@'):
                self.pop()
                right, rs, re = self.parse_unary()
                node = BinOp(left=node, op=_BINOPS[tok.text](), right=right,
                             lineno=ls[0], col_offset=ls[1],
                             end_lineno=re[0], end_col_offset=re[1])
                le = re
            else:
                return (node, ls, le)

    def parse_unary(self):
        tok = self.peek()
        if tok.kind == 'op' and tok.text in _UNARY:
            self.pop()
            operand, os, oe = self.parse_unary()
            node = UnaryOp(op=_UNARY[tok.text](), operand=operand,
                           lineno=tok.srow, col_offset=tok.scol,
                           end_lineno=oe[0], end_col_offset=oe[1])
            return (node, (tok.srow, tok.scol), oe)
        return self.parse_power()

    def parse_power(self):
        node, ls, le = self.parse_postfix()
        if self.at_op('**'):
            self.pop()
            right, rs, re = self.parse_unary()
            node = BinOp(left=node, op=Pow(), right=right, lineno=ls[0],
                         col_offset=ls[1], end_lineno=re[0],
                         end_col_offset=re[1])
            le = re
        return (node, ls, le)

    def parse_postfix(self):
        node, ls, le = self.parse_atom()
        while True:
            if self.at_op('('):
                self.pop()
                args = []
                if not self.at_op(')'):
                    while True:
                        if self.at_op('*') or self.at_op('**'):
                            _no_tree()
                        arg, as_, ae = self.parse_or()
                        args.append(arg)
                        if self.at_name('if') or self.at_name('for'):
                            _no_tree()
                        if self.at_op('='):
                            _no_tree()
                        if self.at_op(','):
                            self.pop()
                            if self.at_op(')'):
                                break
                            continue
                        break
                close_tok = self.expect_op(')')
                node = Call(func=node, args=args, keywords=[], lineno=ls[0],
                            col_offset=ls[1], end_lineno=close_tok.erow,
                            end_col_offset=close_tok.ecol)
                le = (close_tok.erow, close_tok.ecol)
                continue
            if self.at_op('['):
                self.pop()
                if self.at_op(':'):
                    _no_tree()
                index, is_, ie = self.parse_expr(True)
                if self.at_op(':'):
                    _no_tree()
                close_tok = self.expect_op(']')
                node = Subscript(value=node, slice=index, ctx=Load(),
                                 lineno=ls[0], col_offset=ls[1],
                                 end_lineno=close_tok.erow,
                                 end_col_offset=close_tok.ecol)
                le = (close_tok.erow, close_tok.ecol)
                continue
            if self.at_op('.'):
                self.pop()
                tok = self.peek()
                if tok.kind != 'name':
                    _syntax('invalid syntax')
                self.pop()
                node = Attribute(value=node, attr=tok.text, ctx=Load(),
                                 lineno=ls[0], col_offset=ls[1],
                                 end_lineno=tok.erow, end_col_offset=tok.ecol)
                le = (tok.erow, tok.ecol)
                continue
            return (node, ls, le)

    def parse_atom(self):
        tok = self.peek()
        if tok.kind == 'num':
            self.pop()
            node = Constant(value=_number(tok.text, self.source),
                            lineno=tok.srow, col_offset=tok.scol,
                            end_lineno=tok.erow, end_col_offset=tok.ecol)
            return (node, (tok.srow, tok.scol), (tok.erow, tok.ecol))
        if tok.kind == 'name':
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
                _no_tree()
            node = Name(id=word, ctx=Load(), lineno=tok.srow,
                        col_offset=tok.scol, end_lineno=tok.erow,
                        end_col_offset=tok.ecol)
            return (node, (tok.srow, tok.scol), (tok.erow, tok.ecol))
        if tok.kind == 'str' or tok.kind == 'fstr':
            return self.parse_string()
        if self.at_op('...'):
            self.pop()
            node = Constant(value=Ellipsis, lineno=tok.srow,
                            col_offset=tok.scol, end_lineno=tok.erow,
                            end_col_offset=tok.ecol)
            return (node, (tok.srow, tok.scol), (tok.erow, tok.ecol))
        if self.at_op('('):
            open_tok = self.pop()
            if self.at_op(')'):
                close_tok = self.pop()
                node = Tuple(elts=[], ctx=Load(), lineno=open_tok.srow,
                             col_offset=open_tok.scol,
                             end_lineno=close_tok.erow,
                             end_col_offset=close_tok.ecol)
                return (node, (open_tok.srow, open_tok.scol),
                        (close_tok.erow, close_tok.ecol))
            node, ls, le = self.parse_or()
            if self.at_name('for'):
                _no_tree()
            if self.at_op(','):
                elts = [node]
                while self.at_op(','):
                    self.pop()
                    if self.at_op(')'):
                        break
                    elt, es, ee = self.parse_or()
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
        if self.at_op('['):
            open_tok = self.pop()
            elts = []
            if not self.at_op(']'):
                while True:
                    elt, es, ee = self.parse_or()
                    if self.at_name('for'):
                        _no_tree()
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
        if self.at_op('{'):
            open_tok = self.pop()
            if self.at_op('}'):
                close_tok = self.pop()
                node = Dict(keys=[], values=[], lineno=open_tok.srow,
                            col_offset=open_tok.scol,
                            end_lineno=close_tok.erow,
                            end_col_offset=close_tok.ecol)
                return (node, (open_tok.srow, open_tok.scol),
                        (close_tok.erow, close_tok.ecol))
            first, fs, fe = self.parse_or()
            if self.at_op(':'):
                keys = [first]
                values = []
                while True:
                    self.expect_op(':')
                    value, vs, ve = self.parse_or()
                    values.append(value)
                    if self.at_op(','):
                        self.pop()
                        if self.at_op('}'):
                            break
                        key, ks, ke = self.parse_or()
                        keys.append(key)
                        continue
                    break
                close_tok = self.expect_op('}')
                node = Dict(keys=keys, values=values, lineno=open_tok.srow,
                            col_offset=open_tok.scol,
                            end_lineno=close_tok.erow,
                            end_col_offset=close_tok.ecol)
                return (node, (open_tok.srow, open_tok.scol),
                        (close_tok.erow, close_tok.ecol))
            if self.at_name('for'):
                _no_tree()
            elts = [first]
            while self.at_op(','):
                self.pop()
                if self.at_op('}'):
                    break
                elt, es, ee = self.parse_or()
                elts.append(elt)
            close_tok = self.expect_op('}')
            node = Set(elts=elts, lineno=open_tok.srow,
                       col_offset=open_tok.scol,
                       end_lineno=close_tok.erow, end_col_offset=close_tok.ecol)
            return (node, (open_tok.srow, open_tok.scol),
                    (close_tok.erow, close_tok.ecol))
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
        while self.peek().kind == 'str' or self.peek().kind == 'fstr':
            toks.append(self.pop())
        first = toks[0]
        last = toks[-1]
        has_f = False
        for tok in toks:
            if tok.kind == 'fstr':
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
                                        len(tok.quote) == 3)
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
        node = JoinedStr(values=values, lineno=first.srow,
                         col_offset=first.scol, end_lineno=last.erow,
                         end_col_offset=last.ecol)
        return (node, (first.srow, first.scol), (last.erow, last.ecol))

def _type_comment_payload(text):
    # The payload of a type comment, or None when the comment (its
    # source, '#' first, to the end of the line) is not one. CPython's
    # tokenizer matches '#', then any spaces and tabs, then 'type:',
    # then spaces and tabs again; a 'type: ignore' comment is collected
    # apart and is not a type comment.
    i = 1
    n = len(text)
    while i < n and (text[i] == ' ' or text[i] == '\t'):
        i += 1
    if text[i:i + 5] != 'type:':
        return None
    i += 5
    while i < n and (text[i] == ' ' or text[i] == '\t'):
        i += 1
    payload = text[i:]
    if payload[:6] == 'ignore' and (len(payload) == 6 or
                                    (ord(payload[6]) < 128 and
                                     not payload[6].isalnum())):
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

def parse(source, filename='<unknown>', mode='exec', *, type_comments=False,
          feature_version=None, optimize=-1, fold_debug=False):
    if type(source) != type(''):
        _no_tree()
    if mode == 'eval':
        toks = _lex(source, 0, len(source), 1, 0, False)
        parser = _Parser(source, toks, optimize, fold_debug)
        while parser.peek().kind == 'newline':
            parser.pop()
        node, ls, le = parser.parse_expr(False)
        tree = Expression(body=node)
        tree._lumen_tree_source = source
        return tree
    if mode != 'exec':
        _no_tree()
    comments = []
    toks = _lex(source, 0, len(source), 1, 0, False, comments)
    if type_comments:
        _check_type_comments(toks, comments)
    parser = _Parser(source, toks, optimize, fold_debug)
    tree = parser.parse_module()
    tree._lumen_tree_source = source
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
