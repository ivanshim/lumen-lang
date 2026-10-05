# Pattern and Match adapters for CPython v3.14.8 Modules/_sre/sre.c; PSF License.
_native = __sre_native
MAGIC = 20230612
CODESIZE = 4
MAXREPEAT = 4294967295
MAXGROUPS = 1073741823
_TOKEN = object()
_NO_COUNT = object()

def _case_value(i, operation):
    if type(i) is not int:
        i = _index(i)
    if not -2147483648 <= i <= 2147483647:
        raise OverflowError('Python int too large to convert to C int')
    if i < 0:
        return i if operation < 3 else False
    return _native(operation, i)

def ascii_tolower(i):
    return _case_value(i, 1)
def unicode_tolower(i):
    return _case_value(i, 2)
def ascii_iscased(i):
    return _case_value(i, 3)
def unicode_iscased(i):
    return _case_value(i, 4)

def _index(n):
    if type(n) is int:
        return n
    from operator import index
    try:
        return index(n)
    except TypeError:
        if getattr(type(n), '__index__', None) is None:
            raise TypeError("'%s' object cannot be interpreted as an integer" % type(n).__name__)
        raise

def _buffer(value):
    if isinstance(value, str):
        return str(value)
    if isinstance(value, (bytes, bytearray)):
        return str(value, 'latin1')
    try:
        return memoryview(value).tobytes().decode('latin1')
    except TypeError:
        raise TypeError('expected string or bytes-like object, got ' + repr(type(value).__name__))

class Pattern:
    __module__ = 're'
    def __init__(self, token=None, pattern=None, flags=0, code=None, groups=0, groupindex=None, indexgroup=None):
        if token is not _TOKEN:
            raise TypeError("cannot create 're.Pattern' instances")
        self._pattern = pattern
        self._flags = flags
        self._code = code
        self._groups = groups
        self._groupindex = dict(groupindex)
        self._indexgroup = indexgroup
        self._isbytes = not isinstance(pattern, str) if pattern is not None else None
    @property
    def pattern(self):
        return self._pattern
    @property
    def flags(self):
        return self._flags
    @property
    def groups(self):
        return self._groups
    @property
    def groupindex(self):
        from types import MappingProxyType
        return MappingProxyType(self._groupindex)
    def _input(self, string, pos, endpos):
        if type(string) is str and type(pos) is int and type(endpos) is int:
            if self._isbytes is True:
                raise TypeError('cannot use a bytes pattern on a string-like object')
            pos, endpos = _native(9, string, pos, endpos)
            return string, pos, endpos
        text = _buffer(string)
        if self._isbytes is True and isinstance(string, str):
            raise TypeError('cannot use a bytes pattern on a string-like object')
        if self._isbytes is False and not isinstance(string, str):
            raise TypeError('cannot use a string pattern on a bytes-like object')
        pos, endpos = _native(9, text, _index(pos), _index(endpos))
        return text, pos, endpos
    def _run(self, string, text, pos, endpos, mode, must_advance=0):
        state = _native(0, self._code, text, pos, endpos, self._groups, mode, must_advance)
        if state is None:
            return None
        return Match(_TOKEN, self, string, text, pos, endpos, state)
    def prefixmatch(self, string, pos=0, endpos=9223372036854775807):
        text, pos, endpos = self._input(string, pos, endpos)
        return self._run(string, text, pos, endpos, 0)
    match = prefixmatch
    def fullmatch(self, string, pos=0, endpos=9223372036854775807):
        text, pos, endpos = self._input(string, pos, endpos)
        return self._run(string, text, pos, endpos, 1)
    def search(self, string, pos=0, endpos=9223372036854775807):
        text, pos, endpos = self._input(string, pos, endpos)
        return self._run(string, text, pos, endpos, 2)
    def scanner(self, string, pos=0, endpos=9223372036854775807):
        text, pos, endpos = self._input(string, pos, endpos)
        return _Scanner(self, string, text, pos, endpos)
    def finditer(self, string, pos=0, endpos=9223372036854775807):
        return _Iterator(self.scanner(string, pos, endpos))
    def findall(self, string, pos=0, endpos=9223372036854775807):
        if type(string) is str:
            text, pos, endpos = self._input(string, pos, endpos)
            return _native(7, self._code, text, pos, endpos, self.groups)
        empty = b'' if not isinstance(string, str) else ''
        result = []
        for m in self.finditer(string, pos, endpos):
            if self.groups == 0:
                result.append(m.group())
            elif self.groups == 1:
                value = m.group(1)
                result.append(empty if value is None else value)
            else:
                result.append(m.groups(empty))
        return result
    def split(self, string, *args, maxsplit=_NO_COUNT):
        if args:
            if maxsplit is not _NO_COUNT:
                raise TypeError("split() got multiple values for argument 'maxsplit'")
            if len(args) != 1:
                raise TypeError('split() takes at most 2 positional arguments')
            maxsplit = args[0]
        maxsplit = _index(0 if maxsplit is _NO_COUNT else maxsplit)
        text, pos, end = self._input(string, 0, 9223372036854775807)
        source = text.encode('latin1') if not isinstance(string, str) else text
        result = []
        used = 0
        if maxsplit >= 0:
            for m in self.finditer(string):
                if maxsplit and used >= maxsplit:
                    break
                result.append(source[pos:m.start()])
                result.extend(m.groups())
                pos = m.end()
                used += 1
        result.append(source[pos:])
        return result
    def subn(self, repl, string, *args, count=_NO_COUNT):
        if args:
            if count is not _NO_COUNT:
                raise TypeError("subn() got multiple values for argument 'count'")
            if len(args) != 1:
                raise TypeError('subn() takes at most 3 positional arguments')
            count = args[0]
        count = _index(0 if count is _NO_COUNT else count)
        text, pos, end = self._input(string, 0, 9223372036854775807)
        isbytes = not isinstance(string, str)
        if type(string) is str and type(repl) is str and '\\' not in repl and -9223372036854775808 <= count <= 9223372036854775807:
            return _native(8, self._code, text, pos, end, self.groups, count, repl)
        source = text.encode('latin1') if isbytes else text
        empty = b'' if isbytes else ''
        if not callable(repl):
            if isinstance(repl, str) == isbytes:
                raise TypeError('sequence item 0: expected ' + ('a bytes-like object' if isbytes else 'str instance'))
            if isbytes:
                repl = _buffer(repl).encode('latin1')
            else:
                repl = str(repl)
            from re import _compile_template
            replacement = _compile_template(self, repl)
        chunks = []
        used = 0
        if count >= 0:
            for m in self.finditer(string):
                if count and used >= count:
                    break
                value = repl(m) if callable(repl) else replacement.expand(m)
                chunks.append(source[pos:m.start()])
                if value is not None:
                    chunks.append(value)
                pos = m.end()
                used += 1
        chunks.append(source[pos:])
        return empty.join(chunks), used
    def sub(self, repl, string, *args, count=_NO_COUNT):
        if args and count is not _NO_COUNT:
            raise TypeError("sub() got multiple values for argument 'count'")
        return self.subn(repl, string, *args, count=count)[0]
    def __copy__(self):
        return self
    def __deepcopy__(self, memo):
        return self
    def __reduce__(self):
        from re import _compile
        return _compile, (self.pattern, self.flags)
    def __repr__(self):
        # CPython pattern_repr lists flags by bit value, independently of enum declaration order.
        names = ((2, 'IGNORECASE'), (4, 'LOCALE'), (8, 'MULTILINE'),
                 (16, 'DOTALL'), (32, 'UNICODE'), (64, 'VERBOSE'),
                 (128, 'DEBUG'), (256, 'ASCII'))
        remaining = self.flags
        if isinstance(self.pattern, str) and remaining & 292 == 32:
            remaining &= ~32
        parts = []
        for n, name in names:
            if remaining & n:
                parts.append('re.' + name)
                remaining &= ~n
        if remaining:
            parts.append(hex(remaining & 0xffffffff))
        return 're.compile(' + repr(self.pattern)[:200] + (', ' + '|'.join(parts) if parts else '') + ')'
    def __eq__(self, other):
        if not isinstance(other, Pattern):
            return NotImplemented
        return self.pattern == other.pattern and self.flags == other.flags and self._code == other._code
    def __hash__(self):
        return hash((self.pattern, self.flags, tuple(self._code)))
    @classmethod
    def __class_getitem__(cls, item):
        from types import GenericAlias
        return GenericAlias(cls, item)

class Match:
    __module__ = 're'
    def __init__(self, token=None, pattern=None, string=None, text=None, pos=0, endpos=0, state=None):
        if token is not _TOKEN:
            raise TypeError("cannot create 're.Match' instances")
        self._re = pattern
        self._string = string
        self._text = text
        self._pos = pos
        self._endpos = endpos
        self._state = state
    @property
    def re(self):
        return self._re
    @property
    def string(self):
        return self._string
    @property
    def pos(self):
        return self._pos
    @property
    def endpos(self):
        return self._endpos
    @property
    def lastindex(self):
        return None if self._state[3] == -1 else self._state[3]
    @property
    def lastgroup(self):
        return None if self.lastindex is None else self.re._indexgroup[self.lastindex]
    @property
    def regs(self):
        return tuple(self.span(i) for i in range(self.re.groups+1))
    def _index(self, group):
        if isinstance(group, str):
            if group not in self._re._groupindex:
                raise IndexError('no such group')
            return self._re._groupindex[group]
        try:
            if type(group) is not int:
                group = _index(group)
        except TypeError:
            raise IndexError('no such group') from None
        if group < 0 or group > self._re._groups:
            raise IndexError('no such group')
        return group
    def span(self, group=0):
        group = self._index(group)
        if group == 0:
            return self._state[0], self._state[1]
        marks = self._state[2]
        a, b = marks[(group-1)*2:(group-1)*2+2]
        return (-1, -1) if a < 0 or b < a else (a, b)
    def start(self, group=0):
        if type(group) is int and group == 0:
            return self._state[0]
        return self.span(group)[0]
    def end(self, group=0):
        if type(group) is int and group == 0:
            return self._state[1]
        return self.span(group)[1]
    def group(self, *groups):
        if not groups:
            groups = (0,)
        result = []
        source = self._string
        if type(source) is str:
            captures = _native(10, source, self._state, self._re._groups, self._re._groupindex, groups)
            if captures is not None:
                return captures[0]
        for group in groups:
            a, b = self.span(group)
            value = None if a < 0 else (source if type(source) is str else _buffer(source))[a:b]
            if value is not None and not isinstance(source, str):
                value = value.encode('latin1')
            result.append(value)
        return result[0] if len(result) == 1 else tuple(result)
    def __getitem__(self, group):
        return self.group(group)
    def groups(self, default=None):
        return tuple(default if self.start(i) < 0 else self.group(i) for i in range(1, self.re.groups+1))
    def groupdict(self, default=None):
        return {name: default if self.start(i) < 0 else self.group(i) for name, i in self.re._groupindex.items()}
    def expand(self, template):
        from re import _compile_template
        return _compile_template(self.re, template).expand(self)
    def __copy__(self):
        return self
    def __deepcopy__(self, memo):
        return self
    def __repr__(self):
        return '<re.Match object; span=' + repr(self.span()) + ', match=' + repr(self.group()) + '>'
    @classmethod
    def __class_getitem__(cls, item):
        from types import GenericAlias
        return GenericAlias(cls, item)

class _Scanner:
    def __init__(self, pattern, string, text, pos, endpos):
        self.pattern = pattern
        self.string = string
        self.text = text
        self.pos = pos
        self.endpos = endpos
        self.advance = 0
        self.done = False
        self._lease = memoryview(string) if isinstance(string, bytearray) else None
    def _next(self, mode):
        if self.done:
            return None
        m = self.pattern._run(self.string, self.text, self.pos, self.endpos, mode, self.advance)
        if m is None:
            self.done = True
            if self._lease is not None:
                self._lease.release()
                self._lease = None
        else:
            self.advance = int(m.start() == m.end())
            self.pos = m.end()
        return m
    def search(self):
        return self._next(2)
    def prefixmatch(self):
        return self._next(0)
    match = prefixmatch

class _Iterator:
    def __init__(self, scanner):
        self.scanner = scanner
    def __iter__(self):
        return self
    def __next__(self):
        match = self.scanner.search()
        if match is None:
            raise StopIteration
        return match

class _Template:
    def __init__(self, items):
        self.items = items
    def expand(self, match):
        empty = self.items[0][:0]
        result = []
        for i, item in enumerate(self.items):
            if i % 2:
                item = match.group(item)
                if item is None:
                    item = empty
            result.append(item)
        return empty.join(result)

def template(pattern, items):
    if not isinstance(items, list) or len(items) % 2 == 0:
        raise TypeError('invalid template')
    for index in items[1::2]:
        if not isinstance(index, int):
            raise TypeError('an integer is required')
        if index < 0:
            raise TypeError('invalid template')
    return _Template(items)

def compile(pattern, flags, code, groups, groupindex, indexgroup):
    flags = _index(flags)
    if not -2147483648 <= flags <= 2147483647:
        raise OverflowError('Python int too large to convert to C int')
    if not isinstance(code, list):
        actual = 'None' if code is None else type(code).__name__
        raise TypeError("compile() argument 'code' must be list, not " + actual)
    groups = _index(groups)
    if not -9223372036854775808 <= groups <= 9223372036854775807:
        raise OverflowError('Python int too large to convert to C ssize_t')
    if not isinstance(groupindex, dict):
        actual = 'None' if groupindex is None else type(groupindex).__name__
        raise TypeError("compile() argument 'groupindex' must be dict, not " + actual)
    if not isinstance(indexgroup, tuple):
        actual = 'None' if indexgroup is None else type(indexgroup).__name__
        raise TypeError("compile() argument 'indexgroup' must be tuple, not " + actual)
    converted = []
    for op in code:
        if not isinstance(op, int):
            raise TypeError('an integer is required')
        n = op if type(op) is int else int.__index__(op)
        if n < 0:
            raise OverflowError("can't convert negative value to unsigned int")
        if n > 18446744073709551615:
            raise OverflowError('Python int too large to convert to C unsigned long')
        if n > MAXREPEAT:
            raise OverflowError('regular expression code size limit exceeded')
        converted.append(n)
    if pattern is not None:
        _buffer(pattern)
    from _sre_validation import validate
    validate(converted, groups)
    return Pattern(_TOKEN, pattern, flags, converted, groups, groupindex, indexgroup)
