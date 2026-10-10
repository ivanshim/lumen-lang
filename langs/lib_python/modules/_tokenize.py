# Runtime adapter: a Python port of CPython's C tokenizer at v3.14.8 /
# 8e6e75d9102e (Python/Python-tokenize.c, Parser/lexer/lexer.c,
# Parser/lexer/state.c, Parser/tokenizer/helpers.c,
# Parser/tokenizer/readline_tokenizer.c, Parser/token.c); PSF License.
# TokenizerIter yields the same 5-tuples as the C implementation; where the
# C works on UTF-8 bytes and converts byte columns to character columns on
# the way out, this port keeps the decoded text and its columns are
# character columns throughout, which is the same number the C produces.

# The suite's re module is still growing its flag roster, while tokenize.py
# compiles its cookie patterns with re.ASCII.  Supply the standard value
# (CPython Lib/re/__init__.py: ASCII = A = 256) only when re does not name
# it yet; the bit is passed through to re's engine unchanged.
import re as _re
if not hasattr(_re, 'ASCII'):
    _re.ASCII = _re.A = 256

import warnings as _warnings

# Token types (CPython Include/internal/pycore_token.h).
ENDMARKER = 0
NAME = 1
NUMBER = 2
STRING = 3
NEWLINE = 4
INDENT = 5
DEDENT = 6
LPAR = 7
RPAR = 8
LSQB = 9
RSQB = 10
COLON = 11
COMMA = 12
SEMI = 13
PLUS = 14
MINUS = 15
STAR = 16
SLASH = 17
VBAR = 18
AMPER = 19
LESS = 20
GREATER = 21
EQUAL = 22
DOT = 23
PERCENT = 24
LBRACE = 25
RBRACE = 26
EQEQUAL = 27
NOTEQUAL = 28
LESSEQUAL = 29
GREATEREQUAL = 30
TILDE = 31
CIRCUMFLEX = 32
LEFTSHIFT = 33
RIGHTSHIFT = 34
DOUBLESTAR = 35
PLUSEQUAL = 36
MINEQUAL = 37
STAREQUAL = 38
SLASHEQUAL = 39
PERCENTEQUAL = 40
AMPEREQUAL = 41
VBAREQUAL = 42
CIRCUMFLEXEQUAL = 43
LEFTSHIFTEQUAL = 44
RIGHTSHIFTEQUAL = 45
DOUBLESTAREQUAL = 46
DOUBLESLASH = 47
DOUBLESLASHEQUAL = 48
AT = 49
ATEQUAL = 50
RARROW = 51
ELLIPSIS = 52
COLONEQUAL = 53
EXCLAMATION = 54
OP = 55
TYPE_IGNORE = 56
TYPE_COMMENT = 57
SOFT_KEYWORD = 58
FSTRING_START = 59
FSTRING_MIDDLE = 60
FSTRING_END = 61
TSTRING_START = 62
TSTRING_MIDDLE = 63
TSTRING_END = 64
COMMENT = 65
NL = 66
ERRORTOKEN = 67

# Error codes (CPython Include/errcode.h).
E_OK = 10
E_EOF = 11
E_INTR = 12
E_TOKEN = 13
E_NOMEM = 15
E_ERROR = 17
E_TABSPACE = 18
E_TOODEEP = 20
E_DEDENT = 21
E_DECODE = 22
E_EOFS = 23
E_EOLS = 24
E_LINECONT = 25
E_INTERACT_STOP = 28

MAXINDENT = 100         # Max indentation level
MAXLEVEL = 200          # Max parentheses level
MAXFSTRINGLEVEL = 150   # Max f-string nesting level
MAX_EXPR_NESTING = 3
ALTTABSIZE = 1

TOK_REGULAR_MODE = 0
TOK_FSTRING_MODE = 1

FSTRING = 0
TSTRING = 1

_STRINGLIT = (STRING, FSTRING_MIDDLE, TSTRING_MIDDLE)

_ONE_CHAR = {
    '!': EXCLAMATION, '%': PERCENT, '&': AMPER, '(': LPAR, ')': RPAR,
    '*': STAR, '+': PLUS, ',': COMMA, '-': MINUS, '.': DOT, '/': SLASH,
    ':': COLON, ';': SEMI, '<': LESS, '=': EQUAL, '>': GREATER, '@': AT,
    '[': LSQB, ']': RSQB, '^': CIRCUMFLEX, '{': LBRACE, '|': VBAR,
    '}': RBRACE, '~': TILDE,
}

_TWO_CHARS = {
    '!=': NOTEQUAL, '%=': PERCENTEQUAL, '&=': AMPEREQUAL,
    '**': DOUBLESTAR, '*=': STAREQUAL, '+=': PLUSEQUAL, '-=': MINEQUAL,
    '->': RARROW, '//': DOUBLESLASH, '/=': SLASHEQUAL, ':=': COLONEQUAL,
    '<<': LEFTSHIFT, '<=': LESSEQUAL, '<>': NOTEQUAL, '==': EQEQUAL,
    '>=': GREATEREQUAL, '>>': RIGHTSHIFT, '@=': ATEQUAL, '^=': CIRCUMFLEXEQUAL,
    '|=': VBAREQUAL,
}

_THREE_CHARS = {
    '**=': DOUBLESTAREQUAL, '...': ELLIPSIS, '//=': DOUBLESLASHEQUAL,
    '<<=': LEFTSHIFTEQUAL, '>>=': RIGHTSHIFTEQUAL,
}


def _is_digit(c):
    return c is not None and '0' <= c <= '9'


def _is_xdigit(c):
    return c is not None and ('0' <= c <= '9' or 'a' <= c <= 'f' or 'A' <= c <= 'F')


def _is_potential_identifier_start(c):
    return c is not None and (
        'a' <= c <= 'z' or 'A' <= c <= 'Z' or c == '_' or ord(c) >= 128)


def _is_potential_identifier_char(c):
    return c is not None and (
        'a' <= c <= 'z' or 'A' <= c <= 'Z' or '0' <= c <= '9'
        or c == '_' or ord(c) >= 128)


class _TokenizerMode(object):
    def __init__(self):
        self.kind = TOK_REGULAR_MODE
        self.curly_bracket_depth = 0
        self.curly_bracket_expr_start_depth = -1
        self.quote = ''
        self.quote_size = 0
        self.raw = False
        self.start = None
        self.multi_line_start = None
        self.first_line = 0
        self.last_expr_start = None
        self.in_debug = False
        self.in_format_spec = False
        self.string_kind = FSTRING


class _TokState(object):
    def __init__(self, readline, encoding):
        self.buf = ''
        self.cur = 0
        self.inp = 0
        self.start = None
        self.done = E_OK
        self.tabsize = 8
        self.indent = 0
        self.indstack = [0]
        self.altindstack = [0]
        self.atbol = True
        self.pendin = 0
        self.lineno = 0
        self.first_lineno = 0
        self.level = 0
        self.parenstack = []
        self.parenlinenostack = []
        self.cont_line = False
        self.line_start = 0
        self.multi_line_start = 0
        self.encoding = encoding
        self.readline = readline
        self.decoding_erred = False
        self.tok_extra_tokens = False
        self.comment_newline = False
        self.implicit_newline = False
        self.tok_mode_stack = [_TokenizerMode() for _ in range(MAXFSTRINGLEVEL)]
        self.tok_mode_stack_index = 0


def _inside_fstring(tok):
    return tok.tok_mode_stack_index > 0


def _inside_fstring_expr(mode):
    return mode.curly_bracket_expr_start_depth >= 0


def _inside_fstring_expr_at_top(mode):
    return mode.curly_bracket_depth - mode.curly_bracket_expr_start_depth == 1


# Parser/tokenizer/helpers.c: _syntaxerror_range.  Columns passed in are
# character columns already, so the C byte-to-character conversion is the
# identity here.
def _syntaxerror_range(tok, msg, col_offset, end_col_offset):
    errtext = ''.join(tok.bufc[tok.line_start:tok.cur])
    if col_offset == -1:
        col_offset = len(errtext)
    if end_col_offset == -1:
        end_col_offset = col_offset
    rest = ''.join(tok.bufc[tok.line_start:])
    nl = rest.find('\n')
    line_len = nl if nl != -1 else len(rest)
    if line_len != tok.cur - tok.line_start:
        errtext = ''.join(tok.bufc[tok.line_start:tok.line_start + line_len])
    raise SyntaxError(msg, ('<string>', tok.lineno, col_offset, errtext,
                            tok.lineno, end_col_offset))


def _syntaxerror(tok, msg):
    _syntaxerror_range(tok, msg, -1, -1)


# Parser/tokenizer/helpers.c: _PyTokenizer_indenterror.
def _indenterror(tok):
    tok.done = E_TABSPACE
    tok.cur = tok.inp
    return ERRORTOKEN


# Parser/tokenizer/helpers.c: _PyTokenizer_warn_invalid_escape_sequence.
def _warn_invalid_escape_sequence(tok, ch):
    msg = ('"\\%s" is an invalid escape sequence. '
           'Such sequences will not work in the future. '
           'Did you mean "\\\\%s"? A raw string is also an option.' % (ch, ch))
    _warnings.warn_explicit(msg, SyntaxWarning, '<string>', tok.lineno)
    return 0


# Parser/tokenizer/helpers.c: _PyTokenizer_parser_warn (SyntaxWarning only).
def _parser_warn(tok, msg):
    _warnings.warn_explicit(msg, SyntaxWarning, '<string>', tok.lineno)
    return 0


# Parser/tokenizer/readline_tokenizer.c: tok_readline_string and
# tok_underflow_readline.
def _underflow_readline(tok):
    if tok.start is None and not _inside_fstring(tok):
        tok.bufc = []
        tok.line_null = False
        tok.cur = 0
        tok.inp = 0
    try:
        line = tok.readline()
    except StopIteration:
        line = b'' if tok.encoding is not None else ''
    if tok.encoding is not None:
        if not isinstance(line, bytes):
            raise TypeError('readline() returned a non-bytes object')
        line = line.decode(tok.encoding, 'replace')
    else:
        if not isinstance(line, str):
            raise TypeError('readline() returned a non-string object')
    # The character row only ever grows, exactly as the C tokenizer's
    # buffer does, and one character read off it never rescans the text.
    tok.bufc.extend(line)
    tok.inp = len(tok.bufc)
    tok.line_null = '\x00' in line
    tok.line_start = tok.cur
    if tok.inp == tok.cur:
        tok.done = E_EOF
        return 0
    tok.implicit_newline = False
    if tok.bufc[tok.inp - 1] != '\n':
        tok.bufc.append('\n')
        tok.inp += 1
        tok.implicit_newline = True
    tok.lineno += 1
    return 1


# Parser/lexer/lexer.c: tok_nextc.  Returns a one-character str, or None at
# end of input (the C EOF).
def _nextc(tok):
    while True:
        if tok.cur != tok.inp:
            c = tok.bufc[tok.cur]
            tok.cur += 1
            return c
        if tok.done != E_OK:
            return None
        if not _underflow_readline(tok):
            tok.cur = tok.inp
            return None
        tok.line_start = tok.cur
        if tok.line_null:
            _syntaxerror(tok, 'source code cannot contain null bytes')


# Parser/lexer/lexer.c: tok_backup.
def _backup(tok, c):
    if c is not None:
        tok.cur -= 1


# Parser/lexer/lexer.c: lookahead.
def _lookahead(tok, test):
    i = 0
    while True:
        c = _nextc(tok)
        if i == len(test):
            res = not _is_potential_identifier_char(c)
        elif c is not None and c == test[i]:
            i += 1
            continue
        else:
            res = 0
        _backup(tok, c)
        while i > 0:
            i -= 1
            _backup(tok, test[i])
        return res


# Parser/lexer/lexer.c: verify_end_of_number.
def _verify_end_of_number(tok, c, kind):
    if tok.tok_extra_tokens:
        return True
    r = 0
    if c == 'a':
        r = _lookahead(tok, 'nd')
    elif c == 'e':
        r = _lookahead(tok, 'lse')
    elif c == 'f':
        r = _lookahead(tok, 'or')
    elif c == 'i':
        c2 = _nextc(tok)
        if c2 == 'f' or c2 == 'n' or c2 == 's':
            r = 1
        _backup(tok, c2)
    elif c == 'o':
        r = _lookahead(tok, 'r')
    elif c == 'n':
        r = _lookahead(tok, 'ot')
    if r:
        _backup(tok, c)
        _parser_warn(tok, 'invalid %s literal' % kind)
        _nextc(tok)
    elif _is_potential_identifier_char(c) and ord(c) < 128:
        _backup(tok, c)
        _syntaxerror(tok, 'invalid %s literal' % kind)
    return True


# Parser/lexer/lexer.c: verify_identifier (PEP 3131).  Only reached when
# extra tokens are off; the C checks XID_Start/XID_Continue, which
# str.isidentifier spells here.
def _verify_identifier(tok):
    if tok.tok_extra_tokens:
        return True
    s = ''.join(tok.bufc[tok.start:tok.cur])
    invalid = len(s)
    if not s[0].isidentifier():
        invalid = 0
    else:
        for i in range(1, len(s)):
            if not ('a' + s[i]).isidentifier():
                invalid = i
                break
    if invalid < len(s):
        ch = s[invalid]
        tok.cur = tok.start + invalid + 1
        if ch.isprintable():
            _syntaxerror(tok, "invalid character '%s' (U+%04X)" % (ch, ord(ch)))
        else:
            _syntaxerror(tok, 'invalid non-printable character U+%04X' % ord(ch))
    return True


# Parser/lexer/lexer.c: tok_decimal_tail.  Raises on error; the C signals
# the same through a 0 return and a set error.
def _decimal_tail(tok):
    while True:
        c = _nextc(tok)
        while _is_digit(c):
            c = _nextc(tok)
        if c != '_':
            return c
        c = _nextc(tok)
        if not _is_digit(c):
            _backup(tok, c)
            _syntaxerror(tok, 'invalid decimal literal')


# Parser/lexer/lexer.c: tok_continuation_line.  Returns -1 on error with
# tok.done set, like the C.
def _continuation_line(tok):
    c = _nextc(tok)
    if c == '\r':
        c = _nextc(tok)
    if c != '\n':
        tok.done = E_LINECONT
        return -1
    c = _nextc(tok)
    if c is None:
        tok.done = E_EOF
        tok.cur = tok.inp
        return -1
    _backup(tok, c)
    return c


# Parser/lexer/lexer.c: maybe_raise_syntax_error_for_string_prefixes.
def _check_string_prefixes(tok, saw_b, saw_r, saw_u, saw_f, saw_t):
    pairs = ((saw_u, saw_b, 'u', 'b'), (saw_u, saw_r, 'u', 'r'),
             (saw_u, saw_f, 'u', 'f'), (saw_u, saw_t, 'u', 't'),
             (saw_b, saw_f, 'b', 'f'), (saw_b, saw_t, 'b', 't'),
             (saw_f, saw_t, 'f', 't'))
    for first, second, p1, p2 in pairs:
        if first and second:
            _syntaxerror_range(
                tok, "'%s' and '%s' prefixes are incompatible" % (p1, p2),
                tok.start + 1 - tok.line_start, tok.cur - tok.line_start)


# Parser/lexer/lexer.c: the f_string_quote label of tok_get_normal_mode.
def _f_string_quote(tok, c):
    first = tok.bufc[tok.start]
    if first.lower() in ('f', 'r', 't') and (c == "'" or c == '"'):
        quote = c
        quote_size = 1
        tok.first_lineno = tok.lineno
        tok.multi_line_start = tok.line_start
        after_quote = _nextc(tok)
        if after_quote == quote:
            after_after_quote = _nextc(tok)
            if after_after_quote == quote:
                quote_size = 3
            else:
                _backup(tok, after_after_quote)
                _backup(tok, after_quote)
        if after_quote != quote:
            _backup(tok, after_quote)
        p_start = tok.start
        p_end = tok.cur
        if tok.tok_mode_stack_index + 1 >= MAXFSTRINGLEVEL:
            _syntaxerror(tok, 'too many nested f-strings or t-strings')
        tok.tok_mode_stack_index += 1
        mode = tok.tok_mode_stack[tok.tok_mode_stack_index]
        mode.kind = TOK_FSTRING_MODE
        mode.quote = quote
        mode.quote_size = quote_size
        mode.start = tok.start
        mode.multi_line_start = tok.line_start
        mode.first_line = tok.lineno
        mode.last_expr_start = None
        mode.in_format_spec = False
        mode.in_debug = False
        string_kind = FSTRING
        low = first.lower()
        if low == 't':
            mode.raw = tok.bufc[tok.start + 1].lower() == 'r'
            string_kind = TSTRING
        elif low == 'f':
            mode.raw = tok.bufc[tok.start + 1].lower() == 'r'
        else:
            mode.raw = True
            if tok.bufc[tok.start + 1].lower() == 't':
                string_kind = TSTRING
        mode.string_kind = string_kind
        mode.curly_bracket_depth = 0
        mode.curly_bracket_expr_start_depth = -1
        return (TSTRING_START if string_kind == TSTRING else FSTRING_START,
                p_start, p_end)
    return _letter_quote(tok, c)


# Parser/lexer/lexer.c: the letter_quote label of tok_get_normal_mode.
def _letter_quote(tok, c):
    quote = c
    quote_size = 1
    end_quote_size = 0
    has_escaped_quote = False
    tok.first_lineno = tok.lineno
    tok.multi_line_start = tok.line_start
    c = _nextc(tok)
    if c == quote:
        c = _nextc(tok)
        if c == quote:
            quote_size = 3
        else:
            end_quote_size = 1     # empty string found
    if c != quote:
        _backup(tok, c)
    while end_quote_size != quote_size:
        # Ordinary buffered text needs no per-character tokenizer calls.
        cur = tok.cur
        end = tok.inp
        chars = tok.bufc
        for special in (quote, '\\', '\n'):
            try:
                end = chars.index(special, cur, end)
            except ValueError:
                pass
        if end != cur:
            end_quote_size = 0
            tok.cur = end
        c = _nextc(tok)
        if c is None or (quote_size == 1 and c == '\n'):
            tok.cur = tok.start + 1
            tok.line_start = tok.multi_line_start
            start = tok.lineno
            tok.lineno = tok.first_lineno
            if _inside_fstring(tok):
                mode = tok.tok_mode_stack[tok.tok_mode_stack_index]
                if mode.quote == quote and mode.quote_size == quote_size:
                    prefix = 't' if mode.string_kind == TSTRING else 'f'
                    _syntaxerror(tok, "%s-string: expecting '}'" % prefix)
            if quote_size == 3:
                _syntaxerror(tok, 'unterminated triple-quoted string literal'
                             ' (detected at line %d)' % start)
            if has_escaped_quote:
                _syntaxerror(tok, 'unterminated string literal (detected at'
                             ' line %d); perhaps you escaped the end quote?'
                             % start)
            _syntaxerror(tok, 'unterminated string literal (detected at'
                         ' line %d)' % start)
        if c == quote:
            end_quote_size += 1
        else:
            end_quote_size = 0
            if c == '\\':
                c = _nextc(tok)
                if c == quote:
                    has_escaped_quote = True
                if c == '\r':
                    c = _nextc(tok)
    return (STRING, tok.start, tok.cur)


def _make_token(tok, type_, p_start, p_end):
    return (type_, p_start, p_end)


# Parser/lexer/lexer.c: tok_get_normal_mode.  Returns a (type, start, end)
# triple; start/end are buffer indices or None, as the C pointers are.
def _tok_get_normal_mode(tok, current_tok):
    while True:  # nextline
        tok.start = None
        blankline = False
        if tok.atbol:
            col = 0
            altcol = 0
            tok.atbol = False
            cont_line_col = 0
            while True:
                c = _nextc(tok)
                if c == ' ':
                    col += 1
                    altcol += 1
                elif c == '\t':
                    col = (col // tok.tabsize + 1) * tok.tabsize
                    altcol = (altcol // ALTTABSIZE + 1) * ALTTABSIZE
                elif c == '\x0c':
                    col = altcol = 0
                elif c == '\\':
                    if not cont_line_col:
                        cont_line_col = col
                    c = _continuation_line(tok)
                    if c == -1:
                        return (ERRORTOKEN, None, None)
                else:
                    break
            _backup(tok, c)
            if c == '#' or c == '\n' or c == '\r':
                blankline = True
            if not blankline and tok.level == 0:
                if cont_line_col:
                    col = cont_line_col
                    altcol = cont_line_col
                if col == tok.indstack[tok.indent]:
                    if altcol != tok.altindstack[tok.indent]:
                        return (_indenterror(tok), None, None)
                elif col > tok.indstack[tok.indent]:
                    if tok.indent + 1 >= MAXINDENT:
                        tok.done = E_TOODEEP
                        tok.cur = tok.inp
                        return (ERRORTOKEN, None, None)
                    if altcol <= tok.altindstack[tok.indent]:
                        return (_indenterror(tok), None, None)
                    tok.pendin += 1
                    tok.indent += 1
                    # The C keeps a fixed array and overwrites the slot:
                    # indstack[++tok->indent] = col.
                    tok.indstack[tok.indent:] = [col]
                    tok.altindstack[tok.indent:] = [altcol]
                else:
                    while tok.indent > 0 and col < tok.indstack[tok.indent]:
                        tok.pendin -= 1
                        tok.indent -= 1
                    if col != tok.indstack[tok.indent]:
                        tok.done = E_DEDENT
                        tok.cur = tok.inp
                        return (ERRORTOKEN, None, None)
                    if altcol != tok.altindstack[tok.indent]:
                        return (_indenterror(tok), None, None)

        tok.start = tok.cur

        if tok.pendin != 0:
            if tok.pendin < 0:
                if tok.tok_extra_tokens:
                    p_start = tok.cur
                    p_end = tok.cur
                else:
                    p_start = None
                    p_end = None
                tok.pendin += 1
                return (DEDENT, p_start, p_end)
            else:
                if tok.tok_extra_tokens:
                    p_start = 0
                    p_end = tok.cur
                else:
                    p_start = None
                    p_end = None
                tok.pendin -= 1
                return (INDENT, p_start, p_end)

        c = _nextc(tok)
        _backup(tok, c)

        nextline = False
        while True:  # again
            tok.start = None
            c = _nextc(tok)
            while c == ' ' or c == '\t' or c == '\x0c':
                c = _nextc(tok)
            tok.start = tok.cur - 1

            if c == '#':
                while c is not None and c != '\n' and c != '\r':
                    c = _nextc(tok)
                if tok.tok_extra_tokens:
                    _backup(tok, c)
                    tok.comment_newline = blankline
                    return (COMMENT, tok.start, tok.cur)

            if c is None:
                if tok.level:
                    return (ERRORTOKEN, None, None)
                if tok.done == E_EOF:
                    return (ENDMARKER, None, None)
                return (ERRORTOKEN, None, None)

            nonascii = False
            if _is_potential_identifier_start(c):
                saw_b = saw_r = saw_u = saw_f = saw_t = False
                is_string = False
                while True:
                    if not saw_b and (c == 'b' or c == 'B'):
                        saw_b = True
                    elif not saw_u and (c == 'u' or c == 'U'):
                        saw_u = True
                    elif not saw_r and (c == 'r' or c == 'R'):
                        saw_r = True
                    elif not saw_f and (c == 'f' or c == 'F'):
                        saw_f = True
                    elif not saw_t and (c == 't' or c == 'T'):
                        saw_t = True
                    else:
                        break
                    c = _nextc(tok)
                    if c == '"' or c == "'":
                        _check_string_prefixes(tok, saw_b, saw_r, saw_u,
                                               saw_f, saw_t)
                        is_string = True
                        break
                if is_string:
                    if saw_f or saw_t:
                        return _f_string_quote(tok, c)
                    return _letter_quote(tok, c)
                while _is_potential_identifier_char(c):
                    if ord(c) >= 128:
                        nonascii = True
                    c = _nextc(tok)
                _backup(tok, c)
                if nonascii:
                    _verify_identifier(tok)
                return (NAME, tok.start, tok.cur)

            if c == '\r':
                c = _nextc(tok)

            if c == '\n':
                tok.atbol = True
                if blankline or tok.level > 0:
                    if tok.tok_extra_tokens:
                        if tok.comment_newline:
                            tok.comment_newline = False
                        return (NL, tok.start, tok.cur)
                    nextline = True
                    break
                if tok.comment_newline and tok.tok_extra_tokens:
                    tok.comment_newline = False
                    return (NL, tok.start, tok.cur)
                p_start = tok.start
                p_end = tok.cur - 1
                tok.cont_line = False
                return (NEWLINE, p_start, p_end)

            if c == '.':
                c = _nextc(tok)
                if _is_digit(c):
                    return _number_tail_fraction(tok, c)
                elif c == '.':
                    c = _nextc(tok)
                    if c == '.':
                        return (ELLIPSIS, tok.start, tok.cur)
                    else:
                        _backup(tok, c)
                    _backup(tok, '.')
                else:
                    _backup(tok, c)
                return (DOT, tok.start, tok.cur)

            if _is_digit(c):
                if c == '0':
                    c = _nextc(tok)
                    if c == 'x' or c == 'X':
                        c = _nextc(tok)
                        while True:
                            if c == '_':
                                c = _nextc(tok)
                            if not _is_xdigit(c):
                                _backup(tok, c)
                                _syntaxerror(tok, 'invalid hexadecimal literal')
                            c = _nextc(tok)
                            while _is_xdigit(c):
                                c = _nextc(tok)
                            if c != '_':
                                break
                        _verify_end_of_number(tok, c, 'hexadecimal')
                    elif c == 'o' or c == 'O':
                        c = _nextc(tok)
                        while True:
                            if c == '_':
                                c = _nextc(tok)
                            if c is None or c < '0' or c >= '8':
                                if _is_digit(c):
                                    _syntaxerror(tok, "invalid digit '%s' in"
                                                 ' octal literal' % c)
                                else:
                                    _backup(tok, c)
                                    _syntaxerror(tok, 'invalid octal literal')
                            c = _nextc(tok)
                            while c is not None and '0' <= c < '8':
                                c = _nextc(tok)
                            if c != '_':
                                break
                        if _is_digit(c):
                            _syntaxerror(tok, "invalid digit '%s' in octal"
                                         ' literal' % c)
                        _verify_end_of_number(tok, c, 'octal')
                    elif c == 'b' or c == 'B':
                        c = _nextc(tok)
                        while True:
                            if c == '_':
                                c = _nextc(tok)
                            if c != '0' and c != '1':
                                if _is_digit(c):
                                    _syntaxerror(tok, "invalid digit '%s' in"
                                                 ' binary literal' % c)
                                else:
                                    _backup(tok, c)
                                    _syntaxerror(tok, 'invalid binary literal')
                            c = _nextc(tok)
                            while c == '0' or c == '1':
                                c = _nextc(tok)
                            if c != '_':
                                break
                        if _is_digit(c):
                            _syntaxerror(tok, "invalid digit '%s' in binary"
                                         ' literal' % c)
                        _verify_end_of_number(tok, c, 'binary')
                    else:
                        nonzero = False
                        while True:
                            if c == '_':
                                c = _nextc(tok)
                                if not _is_digit(c):
                                    _backup(tok, c)
                                    _syntaxerror(tok, 'invalid decimal literal')
                            if c != '0':
                                break
                            c = _nextc(tok)
                        zeros_end = tok.cur
                        if _is_digit(c):
                            nonzero = True
                            c = _decimal_tail(tok)
                        if c == '.':
                            c = _nextc(tok)
                            return _number_tail_fraction(tok, c)
                        elif c == 'e' or c == 'E':
                            return _number_tail_exponent(tok, c)
                        elif c == 'j' or c == 'J':
                            c = _nextc(tok)
                            _verify_end_of_number(tok, c, 'imaginary')
                        elif nonzero and not tok.tok_extra_tokens:
                            _backup(tok, c)
                            _syntaxerror_range(
                                tok, 'leading zeros in decimal integer'
                                ' literals are not permitted; use an 0o'
                                ' prefix for octal integers',
                                tok.start + 1 - tok.line_start,
                                zeros_end - tok.line_start)
                        else:
                            _verify_end_of_number(tok, c, 'decimal')
                    _backup(tok, c)
                    return (NUMBER, tok.start, tok.cur)
                else:
                    c = _decimal_tail(tok)
                    if c == '.':
                        c = _nextc(tok)
                        return _number_tail_fraction(tok, c)
                    return _number_tail_exp(tok, c)

            if c == "'" or c == '"':
                return _letter_quote(tok, c)

            if c == '\\':
                c = _continuation_line(tok)
                if c == -1:
                    return (ERRORTOKEN, None, None)
                tok.cont_line = True
                continue  # again

            is_punctuation = c == ':' or c == '}' or c == '!' or c == '{'
            if is_punctuation and _inside_fstring(tok) \
                    and _inside_fstring_expr(current_tok):
                cursor = current_tok.curly_bracket_depth - (0 if c == '{' else 1)
                in_format_spec = current_tok.in_format_spec
                cursor_in_format_with_debug = cursor == 1 and (
                    current_tok.in_debug or in_format_spec)
                cursor_valid = cursor == 0 or cursor_in_format_with_debug
                # The C calls set_ftstring_expr here; it only fills token
                # metadata the iterator never surfaces, so there is nothing
                # to do for it.
                if c == ':' and cursor == current_tok.curly_bracket_expr_start_depth:
                    current_tok.kind = TOK_FSTRING_MODE
                    current_tok.in_format_spec = True
                    return (_ONE_CHAR[c], tok.start, tok.cur)

            c2 = _nextc(tok)
            two = None
            if c is not None and c2 is not None:
                two = _TWO_CHARS.get(c + c2)
                if c == '<' and c2 == '>':
                    two = OP
            if two is not None and two != OP:
                c3 = _nextc(tok)
                three = None
                if c3 is not None:
                    three = _THREE_CHARS.get(c + c2 + c3)
                if three is not None:
                    two = three
                else:
                    _backup(tok, c3)
                return (two, tok.start, tok.cur)
            _backup(tok, c2)

            if c == '(' or c == '[' or c == '{':
                if tok.level >= MAXLEVEL:
                    _syntaxerror(tok, 'too many nested parentheses')
                tok.parenstack.append(c)
                tok.parenlinenostack.append(tok.lineno)
                tok.level += 1
                if _inside_fstring(tok):
                    current_tok.curly_bracket_depth += 1
            elif c == ')' or c == ']' or c == '}':
                if _inside_fstring(tok) \
                        and not current_tok.curly_bracket_depth and c == '}':
                    prefix = 't' if current_tok.string_kind == TSTRING else 'f'
                    _syntaxerror(tok, "%s-string: single '}' is not allowed"
                                 % prefix)
                if not tok.tok_extra_tokens and not tok.level:
                    _syntaxerror(tok, "unmatched '%s'" % c)
                if tok.level > 0:
                    tok.level -= 1
                    opening = tok.parenstack.pop()
                    opening_lineno = tok.parenlinenostack.pop()
                    if not tok.tok_extra_tokens and not (
                            (opening == '(' and c == ')')
                            or (opening == '[' and c == ']')
                            or (opening == '{' and c == '}')):
                        if _inside_fstring(tok) and opening == '{':
                            previous_bracket = current_tok.curly_bracket_depth - 1
                            if previous_bracket == current_tok.curly_bracket_expr_start_depth:
                                prefix = ('t' if current_tok.string_kind == TSTRING
                                          else 'f')
                                _syntaxerror(tok, "%s-string: unmatched '%s'"
                                             % (prefix, c))
                        if opening_lineno != tok.lineno:
                            _syntaxerror(tok, "closing parenthesis '%s' does"
                                         ' not match opening parenthesis'
                                         " '%s' on line %d"
                                         % (c, opening, opening_lineno))
                        else:
                            _syntaxerror(tok, "closing parenthesis '%s' does"
                                         ' not match opening parenthesis'
                                         " '%s'" % (c, opening))
                if _inside_fstring(tok):
                    current_tok.curly_bracket_depth -= 1
                    if current_tok.curly_bracket_depth < 0:
                        prefix = ('t' if current_tok.string_kind == TSTRING
                                  else 'f')
                        _syntaxerror(tok, "%s-string: unmatched '%s'"
                                     % (prefix, c))
                    if c == '}' and current_tok.curly_bracket_depth == \
                            current_tok.curly_bracket_expr_start_depth:
                        current_tok.curly_bracket_expr_start_depth -= 1
                        current_tok.kind = TOK_FSTRING_MODE
                        current_tok.in_format_spec = False
                        current_tok.in_debug = False

            if c is not None and ord(c) < 128 and not (32 <= ord(c) < 127):
                _syntaxerror(tok, 'invalid non-printable character U+%04X'
                             % ord(c))

            if c == '=' and _inside_fstring_expr_at_top(current_tok):
                current_tok.in_debug = True

            return (_ONE_CHAR.get(c, OP), tok.start, tok.cur)

        if nextline:
            continue


# Parser/lexer/lexer.c: the fraction: label shared by the number paths.
def _number_tail_fraction(tok, c):
    if _is_digit(c):
        c = _decimal_tail(tok)
    return _number_tail_exp(tok, c)


def _number_tail_exp(tok, c):
    if c == 'e' or c == 'E':
        return _number_tail_exponent(tok, c)
    return _number_tail_imag(tok, c)


def _number_tail_exponent(tok, c):
    e = c
    c = _nextc(tok)
    if c == '+' or c == '-':
        c = _nextc(tok)
        if not _is_digit(c):
            _backup(tok, c)
            _syntaxerror(tok, 'invalid decimal literal')
    elif not _is_digit(c):
        _backup(tok, c)
        _verify_end_of_number(tok, e, 'decimal')
        _backup(tok, e)
        return (NUMBER, tok.start, tok.cur)
    c = _decimal_tail(tok)
    return _number_tail_imag(tok, c)


def _number_tail_imag(tok, c):
    if c == 'j' or c == 'J':
        c = _nextc(tok)
        _verify_end_of_number(tok, c, 'imaginary')
    else:
        _verify_end_of_number(tok, c, 'decimal')
    _backup(tok, c)
    return (NUMBER, tok.start, tok.cur)


# Parser/lexer/lexer.c: tok_get_fstring_mode.
def _tok_get_fstring_mode(tok, current_tok):
    tok.start = tok.cur
    tok.first_lineno = tok.lineno

    start_char = _nextc(tok)
    if start_char == '{':
        peek1 = _nextc(tok)
        _backup(tok, peek1)
        if peek1 != '{':
            current_tok.last_expr_start = tok.cur
        _backup(tok, start_char)
        if peek1 != '{':
            current_tok.curly_bracket_expr_start_depth += 1
            if current_tok.curly_bracket_expr_start_depth >= MAX_EXPR_NESTING:
                prefix = ('t' if current_tok.string_kind == TSTRING else 'f')
                _syntaxerror(tok, '%s-string: expressions nested too deeply'
                             % prefix)
            tok.tok_mode_stack[tok.tok_mode_stack_index].kind = TOK_REGULAR_MODE
            return _tok_get_normal_mode(tok, current_tok)
    else:
        _backup(tok, start_char)

    for _ in range(current_tok.quote_size):
        quote = _nextc(tok)
        if quote != current_tok.quote:
            _backup(tok, quote)
            break
    else:
        tok.tok_mode_stack_index -= 1
        if current_tok.string_kind == TSTRING:
            return (TSTRING_END, tok.start, tok.cur)
        return (FSTRING_END, tok.start, tok.cur)

    # f_string_middle:
    tok.multi_line_start = tok.line_start
    end_quote_size = 0
    unicode_escape = False
    while end_quote_size != current_tok.quote_size:
        c = _nextc(tok)
        in_format_spec = current_tok.in_format_spec \
            and _inside_fstring_expr(current_tok)
        if c is None or (current_tok.quote_size == 1 and c == '\n'):
            if in_format_spec and c == '\n':
                if current_tok.quote_size == 1:
                    prefix = ('t' if current_tok.string_kind == TSTRING
                              else 'f')
                    _syntaxerror(tok, '%s-string: newlines are not allowed'
                                 ' in format specifiers for single quoted'
                                 ' %s-strings' % (prefix, prefix))
                _backup(tok, c)
                tok.tok_mode_stack[tok.tok_mode_stack_index].kind = TOK_REGULAR_MODE
                current_tok.in_format_spec = False
                if current_tok.string_kind == TSTRING:
                    return (TSTRING_MIDDLE, tok.start, tok.cur)
                return (FSTRING_MIDDLE, tok.start, tok.cur)
            tok.cur = current_tok.start + 1
            tok.line_start = current_tok.multi_line_start
            start = tok.lineno
            mode = tok.tok_mode_stack[tok.tok_mode_stack_index]
            tok.lineno = mode.first_line
            prefix = 't' if current_tok.string_kind == TSTRING else 'f'
            if current_tok.quote_size == 3:
                _syntaxerror(tok, 'unterminated triple-quoted %s-string'
                             ' literal (detected at line %d)'
                             % (prefix, start))
            else:
                _syntaxerror(tok, 'unterminated %s-string literal (detected'
                             ' at line %d)' % (prefix, start))
        if c == current_tok.quote:
            end_quote_size += 1
            continue
        else:
            end_quote_size = 0

        if c == '{':
            peek = _nextc(tok)
            if peek != '{' or in_format_spec:
                _backup(tok, peek)
                current_tok.last_expr_start = tok.cur
                _backup(tok, c)
                current_tok.curly_bracket_expr_start_depth += 1
                if current_tok.curly_bracket_expr_start_depth >= MAX_EXPR_NESTING:
                    prefix = ('t' if current_tok.string_kind == TSTRING
                              else 'f')
                    _syntaxerror(tok, '%s-string: expressions nested too'
                                 ' deeply' % prefix)
                tok.tok_mode_stack[tok.tok_mode_stack_index].kind = TOK_REGULAR_MODE
                current_tok.in_format_spec = False
                p_end = tok.cur
            else:
                p_end = tok.cur - 1
            if current_tok.string_kind == TSTRING:
                return (TSTRING_MIDDLE, tok.start, p_end)
            return (FSTRING_MIDDLE, tok.start, p_end)
        elif c == '}':
            if unicode_escape:
                if current_tok.string_kind == TSTRING:
                    return (TSTRING_MIDDLE, tok.start, tok.cur)
                return (FSTRING_MIDDLE, tok.start, tok.cur)
            peek = _nextc(tok)
            cursor = current_tok.curly_bracket_depth
            if peek == '}' and not in_format_spec and cursor == 0:
                p_end = tok.cur - 1
            else:
                _backup(tok, peek)
                _backup(tok, c)
                tok.tok_mode_stack[tok.tok_mode_stack_index].kind = TOK_REGULAR_MODE
                current_tok.in_format_spec = False
                p_end = tok.cur
            if current_tok.string_kind == TSTRING:
                return (TSTRING_MIDDLE, tok.start, p_end)
            return (FSTRING_MIDDLE, tok.start, p_end)
        elif c == '\\':
            peek = _nextc(tok)
            if peek == '\r':
                peek = _nextc(tok)
            if peek == '{' or peek == '}':
                if not current_tok.raw:
                    _warn_invalid_escape_sequence(tok, peek)
                _backup(tok, peek)
                continue
            if not current_tok.raw:
                if peek == 'N':
                    peek = _nextc(tok)
                    if peek == '{':
                        unicode_escape = True
                    else:
                        _backup(tok, peek)

    # Backup the f-string quotes to emit a final FSTRING_MIDDLE and add the
    # quotes to the FSTRING_END in the next tokenizer iteration.
    for _ in range(current_tok.quote_size):
        _backup(tok, current_tok.quote)
    if current_tok.string_kind == TSTRING:
        return (TSTRING_MIDDLE, tok.start, tok.cur)
    return (FSTRING_MIDDLE, tok.start, tok.cur)


# Parser/lexer/lexer.c: tok_get and _PyTokenizer_Get.
def _tokenizer_get(tok):
    current_tok = tok.tok_mode_stack[tok.tok_mode_stack_index]
    if current_tok.kind == TOK_REGULAR_MODE:
        return _tok_get_normal_mode(tok, current_tok)
    return _tok_get_fstring_mode(tok, current_tok)


class TokenizerIter(object):
    # Python/Python-tokenize.c: tokenizeriter_new and tokenizeriter_next.
    def __init__(self, readline, *, extra_tokens=False, encoding=None):
        self.tok = _TokState(readline, encoding)
        if extra_tokens:
            self.tok.tok_extra_tokens = True
        self.done = False
        # Line cache, as in the C tokenizeriterobject.
        self.last_line = None
        self.last_lineno = 0

    def __iter__(self):
        return self

    # Python/Python-tokenize.c: _tokenizer_error, for the error codes the
    # lexer reports through tok.done rather than an exception.
    def _tokenizer_error(self):
        tok = self.tok
        errtype = SyntaxError
        if tok.done == E_EOF:
            # PyErr_SetString + PyErr_SyntaxLocationObject: the location is
            # attached attribute by attribute, and the column the C passes
            # is a byte count into the buffer.
            err = SyntaxError('unexpected EOF in multi-line statement')
            err.filename = '<string>'
            err.lineno = tok.lineno
            offset = tok.inp
            if offset < 0:
                offset = 0
            err.offset = len(''.join(tok.bufc[:offset]).encode('utf-8'))
            err.end_lineno = tok.lineno
            raise err
        if tok.done == E_TOKEN:
            msg = 'invalid token'
        elif tok.done == E_DEDENT:
            msg = 'unindent does not match any outer indentation level'
            errtype = IndentationError
        elif tok.done == E_TABSPACE:
            errtype = TabError
            msg = 'inconsistent use of tabs and spaces in indentation'
        elif tok.done == E_TOODEEP:
            errtype = IndentationError
            msg = 'too many levels of indentation'
        elif tok.done == E_LINECONT:
            msg = 'unexpected character after line continuation character'
        elif tok.done == E_NOMEM:
            raise MemoryError()
        elif tok.done == E_INTR:
            raise KeyboardInterrupt()
        else:
            msg = 'unknown tokenization error'
        # The C decodes the line (without its trailing newline) and counts
        # one character past its end for the column.
        error_line = ''.join(tok.bufc[:tok.inp - 1])
        offset = len(error_line) + 1
        raise errtype(msg, ('<string>', tok.lineno, offset, error_line,
                            None, None))

    def __next__(self):
        tok = self.tok
        type_, tstart, tend = _tokenizer_get(tok)
        if type_ == ERRORTOKEN:
            self._tokenizer_error()
        if self.done:
            raise StopIteration('EOF')
        if tstart is None or tend is None:
            string = ''
        else:
            string = ''.join(tok.bufc[tstart:tend])

        is_trailing_token = type_ == ENDMARKER or (
            type_ == DEDENT and tok.done == E_EOF)

        if type_ in _STRINGLIT:
            line_start = tok.multi_line_start
        else:
            line_start = tok.line_start
        if tok.tok_extra_tokens and is_trailing_token:
            line = ''
        else:
            size = tok.inp - line_start
            if size >= 1 and tok.implicit_newline:
                size -= 1
            # _get_current_line: the line is fetched once per tok.lineno and
            # then served from the cache.
            if tok.lineno != self.last_lineno:
                line = ''.join(tok.bufc[line_start:line_start + size])
                self.last_line = line
            else:
                line = self.last_line
            if line is None:
                # The C returns NULL here without setting an error, which
                # the iterator protocol turns into a plain StopIteration.
                raise StopIteration()

        if type_ in _STRINGLIT:
            lineno = tok.first_lineno
        else:
            lineno = tok.lineno
        end_lineno = tok.lineno
        col_offset = -1
        end_col_offset = -1
        if tstart is not None and tstart >= line_start:
            col_offset = tstart - line_start
        if tend is not None and tend >= tok.line_start:
            if lineno == end_lineno:
                end_col_offset = col_offset + (tend - tstart)
            else:
                end_col_offset = tend - tok.line_start
        # _get_col_offsets records the line numbers for the cache check.
        self.last_lineno = lineno

        if tok.tok_extra_tokens:
            if is_trailing_token:
                lineno = end_lineno = lineno + 1
                col_offset = end_col_offset = 0
            if DEDENT < type_ < OP:
                type_ = OP
            elif type_ == NEWLINE:
                if not tok.implicit_newline:
                    if tok.bufc[tok.start] == '\r':
                        string = '\r\n'
                    else:
                        string = '\n'
                end_col_offset += 1
            elif type_ == NL:
                if tok.implicit_newline:
                    string = ''

        if type_ == ENDMARKER:
            self.done = True
        return (type_, string, (lineno, col_offset), (end_lineno, end_col_offset), line)


def _decode_source(source_bytes):
    # CPython v3.14.8 Lib/importlib/_bootstrap_external.py: decode_source.
    import tokenize
    import _io
    source_bytes_readline = _io.BytesIO(source_bytes).readline
    encoding = tokenize.detect_encoding(source_bytes_readline)
    newline_decoder = _io.IncrementalNewlineDecoder(None, True)
    return newline_decoder.decode(source_bytes.decode(encoding[0]))
