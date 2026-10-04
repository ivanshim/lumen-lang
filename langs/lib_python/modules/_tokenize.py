# Python implementation of the native tokenizer interface; PSF License.
# The scanner derives from CPython v3.11.0 Lib/tokenize.py. Public tokenize.py
# remains unchanged at the suite pin; this module implements its C interface.
from token import *

def _scan(readline, encoding):
    from tokenize import TokenInfo, TokenError, _compile, PseudoToken, triple_quoted, single_quoted, endpats
    from tokenize import tabsize
    lnum = parenlev = continued = 0
    numchars = '0123456789'
    contstr, needcont = '', 0
    contline = None
    indents = [0]

    if encoding is not None:
        if encoding == "utf-8-sig":
            # BOM will already have been stripped.
            encoding = "utf-8"
        yield TokenInfo(ENCODING, encoding, (0, 0), (0, 0), '')
    last_line = b''
    line = b''
    while True:                                # loop over lines in stream
        try:
            # We capture the value of the line variable here because
            # readline uses the empty string '' to signal end of input,
            # hence `line` itself will always be overwritten at the end
            # of this loop.
            last_line = line
            line = readline()
        except StopIteration:
            line = b''

        if encoding is not None:
            line = line.decode(encoding)
        lnum += 1
        pos, max = 0, len(line)

        if contstr:                            # continued string
            if not line:
                raise TokenError("EOF in multi-line string", strstart)
            endmatch = endprog.match(line)
            if endmatch:
                pos = end = endmatch.end(0)
                yield TokenInfo(STRING, contstr + line[:end],
                       strstart, (lnum, end), contline + line)
                contstr, needcont = '', 0
                contline = None
            elif needcont and line[-2:] != '\\\n' and line[-3:] != '\\\r\n':
                yield TokenInfo(ERRORTOKEN, contstr + line,
                           strstart, (lnum, len(line)), contline)
                contstr = ''
                contline = None
                continue
            else:
                contstr = contstr + line
                contline = contline + line
                continue

        elif parenlev == 0 and not continued:  # new statement
            if not line: break
            column = 0
            while pos < max:                   # measure leading whitespace
                if line[pos] == ' ':
                    column += 1
                elif line[pos] == '\t':
                    column = (column//tabsize + 1)*tabsize
                elif line[pos] == '\f':
                    column = 0
                else:
                    break
                pos += 1
            if pos == max:
                break

            if line[pos] in '#\r\n':           # skip comments or blank lines
                if line[pos] == '#':
                    comment_token = line[pos:].rstrip('\r\n')
                    yield TokenInfo(COMMENT, comment_token,
                           (lnum, pos), (lnum, pos + len(comment_token)), line)
                    pos += len(comment_token)

                yield TokenInfo(NL, line[pos:],
                           (lnum, pos), (lnum, len(line)), line)
                continue

            if column > indents[-1]:           # count indents or dedents
                indents.append(column)
                yield TokenInfo(INDENT, line[:pos], (lnum, 0), (lnum, pos), line)
            while column < indents[-1]:
                if column not in indents:
                    raise IndentationError(
                        "unindent does not match any outer indentation level",
                        ("<tokenize>", lnum, pos, line))
                indents = indents[:-1]

                yield TokenInfo(DEDENT, '', (lnum, pos), (lnum, pos), line)

        else:                                  # continued statement
            if not line:
                raise TokenError("EOF in multi-line statement", (lnum, 0))
            continued = 0

        while pos < max:
            pseudomatch = _compile(PseudoToken).match(line, pos)
            if pseudomatch:                                # scan for tokens
                start, end = pseudomatch.span(1)
                spos, epos, pos = (lnum, start), (lnum, end), end
                if start == end:
                    continue
                token, initial = line[start:end], line[start]

                if (initial in numchars or                 # ordinary number
                    (initial == '.' and token != '.' and token != '...')):
                    yield TokenInfo(NUMBER, token, spos, epos, line)
                elif initial in '\r\n':
                    if parenlev > 0:
                        yield TokenInfo(NL, token, spos, epos, line)
                    else:
                        yield TokenInfo(NEWLINE, token, spos, epos, line)

                elif initial == '#':
                    assert not token.endswith("\n")
                    yield TokenInfo(COMMENT, token, spos, epos, line)

                elif token in triple_quoted:
                    endprog = _compile(endpats[token])
                    endmatch = endprog.match(line, pos)
                    if endmatch:                           # all on one line
                        pos = endmatch.end(0)
                        token = line[start:pos]
                        yield TokenInfo(STRING, token, spos, (lnum, pos), line)
                    else:
                        strstart = (lnum, start)           # multiple lines
                        contstr = line[start:]
                        contline = line
                        break

                # Check up to the first 3 chars of the token to see if
                #  they're in the single_quoted set. If so, they start
                #  a string.
                # We're using the first 3, because we're looking for
                #  "rb'" (for example) at the start of the token. If
                #  we switch to longer prefixes, this needs to be
                #  adjusted.
                # Note that initial == token[:1].
                # Also note that single quote checking must come after
                #  triple quote checking (above).
                elif (initial in single_quoted or
                      token[:2] in single_quoted or
                      token[:3] in single_quoted):
                    if token[-1] == '\n':                  # continued string
                        strstart = (lnum, start)
                        # Again, using the first 3 chars of the
                        #  token. This is looking for the matching end
                        #  regex for the correct type of quote
                        #  character. So it's really looking for
                        #  endpats["'"] or endpats['"'], by trying to
                        #  skip string prefix characters, if any.
                        endprog = _compile(endpats.get(initial) or
                                           endpats.get(token[1]) or
                                           endpats.get(token[2]))
                        contstr, needcont = line[start:], 1
                        contline = line
                        break
                    else:                                  # ordinary string
                        yield TokenInfo(STRING, token, spos, epos, line)

                elif initial.isidentifier():               # ordinary name
                    yield TokenInfo(NAME, token, spos, epos, line)
                elif initial == '\\':                      # continued stmt
                    continued = 1
                else:
                    if initial in '([{':
                        parenlev += 1
                    elif initial in ')]}':
                        parenlev -= 1
                    yield TokenInfo(OP, token, spos, epos, line)
            else:
                yield TokenInfo(ERRORTOKEN, line[pos],
                           (lnum, pos), (lnum, pos+1), line)
                pos += 1

    # Add an implicit NEWLINE if the input doesn't end in one
    if last_line and last_line[-1] not in '\r\n' and not last_line.strip().startswith("#"):
        yield TokenInfo(NEWLINE, '', (lnum - 1, len(last_line)), (lnum - 1, len(last_line) + 1), '')
    for indent in indents[1:]:                 # pop remaining indent levels
        yield TokenInfo(DEDENT, '', (lnum, 0), (lnum, 0), '')
    yield TokenInfo(ENDMARKER, '', (lnum, 0), (lnum, 0), '')


class TokenizerIter:
    def __init__(self, source, /, *, encoding=None, extra_tokens=False):
        if isinstance(source, str):
            from io import StringIO
            source = StringIO(source).readline
        elif not callable(source):
            raise TypeError('TokenizerIter() argument must be str or callable')
        self._tokens = self._iterate(source, encoding, extra_tokens)

    def __iter__(self):
        return self

    def __next__(self):
        return next(self._tokens)

    def _iterate(self, source, encoding, extra):
        def readline():
            line = source()
            if isinstance(line, bytes):
                return line.decode(encoding or 'utf-8')
            if not isinstance(line, str):
                raise TypeError('readline() returned a non-string')
            return line
        for item in _scan(readline, None):
            kind, text, start, end, line = item
            if kind == STRING and any(prefix in _prefix(text) for prefix in ('f', 't')):
                yield from _interpolated(text, start, line, extra)
                continue
            if not extra:
                if kind in (COMMENT, NL):
                    continue
                if kind == OP:
                    kind = EXACT_TOKEN_TYPES[text]
            yield kind, text, start, end, line

def _prefix(text):
    for position, character in enumerate(text):
        if character in "\"'":
            return text[:position].lower()
    return ''


def _interpolated(text, start, line, extra):
    return _Interpolation(text, start, line, extra).tokens()


class _Interpolation:
    def __init__(self, text, start, line, extra):
        self.text = text
        self.start = start
        self.line = line
        self.extra = extra
        self.output = []
        opening = 0
        while text[opening] not in "\"'":
            opening += 1
        self.prefix = text[:opening].lower()
        quote = text[opening]
        self.width = 3 if text[opening:opening+3] == quote*3 else 1
        self.position = opening + self.width
        self.limit = len(text) - self.width
        self.middle = TSTRING_MIDDLE if 't' in self.prefix else FSTRING_MIDDLE

    def location(self, at):
        before = self.text[:at]
        lines = before.count('\n')
        return (self.start[0] + lines,
                len(before.rsplit('\n', 1)[-1]) if lines else self.start[1] + at)

    def emit(self, kind, word, left, right):
        self.output.append((kind, word, self.location(left), self.location(right), self.line))

    def operator(self, word, at):
        self.emit(OP if self.extra else EXACT_TOKEN_TYPES[word], word, at, at+1)

    def tokens(self):
        beginning = TSTRING_START if 't' in self.prefix else FSTRING_START
        ending = TSTRING_END if 't' in self.prefix else FSTRING_END
        self.emit(beginning, self.text[:self.position], 0, self.position)
        self.literal(False)
        self.emit(ending, self.text[self.limit:], self.limit, len(self.text))
        return self.output

    def literal(self, format_spec):
        left = self.position
        while self.position < self.limit:
            at = self.position
            char = self.text[at]
            if char == '}' and format_spec:
                break
            if not format_spec and self.text[at:at+2] in ('{{', '}}'):
                self.emit(self.middle, self.text[left:at]+char, left, at+1)
                self.position += 2
                left = self.position
                continue
            if char == '{':
                if at > left:
                    self.emit(self.middle, self.text[left:at], left, at)
                self.replacement()
                left = self.position
                continue
            if char == '}':
                raise SyntaxError("f-string: single '}' is not allowed")
            self.position += 1
        if self.position > left:
            self.emit(self.middle, self.text[left:self.position], left, self.position)
        if format_spec and self.position >= self.limit:
            raise SyntaxError("f-string: expecting '}'")

    def replacement(self):
        self.operator('{', self.position)
        self.position += 1
        begin = self.position
        nesting = []
        quote = None
        quote_width = 0
        while self.position < self.limit:
            at = self.position
            char = self.text[at]
            if quote is not None:
                if char == '\\':
                    self.position += 2
                    continue
                if self.text[at:at+quote_width] == quote*quote_width:
                    self.position += quote_width
                    quote = None
                    continue
            elif char in "\"'":
                quote = char
                quote_width = 3 if self.text[at:at+3] == char*3 else 1
                self.position += quote_width
                continue
            elif char in '([{':
                nesting.append(char)
            elif char in ')]}':
                if not nesting and char == '}':
                    break
                if nesting:
                    nesting.pop()
            elif not nesting and (char == ':' or char == '!' and self.text[at:at+2] != '!='):
                break
            self.position += 1
        end = self.position
        expression = self.text[begin:end]
        expression_lines = expression.splitlines(keepends=True)
        for kind, word, left, right, ignored in TokenizerIter(expression, extra_tokens=self.extra):
            if kind in (NEWLINE, ENDMARKER, INDENT, DEDENT):
                continue
            def offset(point):
                return begin + sum(len(part) for part in expression_lines[:point[0]-1]) + point[1]
            self.emit(kind, word, offset(left), offset(right))
        if self.position < self.limit and self.text[self.position] == '!':
            self.operator('!', self.position)
            self.position += 1
            if self.position >= self.limit or self.text[self.position] not in 'sra':
                raise SyntaxError("f-string: invalid conversion character")
            self.emit(NAME, self.text[self.position], self.position, self.position+1)
            self.position += 1
        if self.position < self.limit and self.text[self.position] == ':':
            self.operator(':', self.position)
            self.position += 1
            self.literal(True)
        if self.position >= self.limit or self.text[self.position] != '}':
            raise SyntaxError("f-string: expecting '}'")
        self.operator('}', self.position)
        self.position += 1
