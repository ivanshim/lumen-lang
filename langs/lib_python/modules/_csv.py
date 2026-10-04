# The CSV accelerator, written out as the state machine Modules/_csv.c
# carries. Derived from CPython Modules/_csv.c at v3.14.8 / 8e6e75d9102e;
# PSF License.

QUOTE_MINIMAL = 0
QUOTE_ALL = 1
QUOTE_NONNUMERIC = 2
QUOTE_NONE = 3
QUOTE_STRINGS = 4
QUOTE_NOTNULL = 5

_QUOTE_STYLES = (QUOTE_MINIMAL, QUOTE_ALL, QUOTE_NONNUMERIC, QUOTE_NONE,
                 QUOTE_STRINGS, QUOTE_NOTNULL)

# A value no caller may pass, standing for "this parameter was not given".
_NOT_SET = object()

# The end-of-line sentinel handed to the state machine once per input line.
_EOL = object()

START_RECORD = 0
START_FIELD = 1
ESCAPED_CHAR = 2
IN_FIELD = 3
IN_QUOTED_FIELD = 4
ESCAPE_IN_QUOTED_FIELD = 5
QUOTE_IN_QUOTED_FIELD = 6
EAT_CRNL = 7
AFTER_ESCAPED_CRNL = 8

_dialects = {}

_field_limit = 128 * 1024


class Error(Exception):
    pass


def _type_name(value):
    return type(value).__name__


def _set_char(name, value, default):
    if value is _NOT_SET:
        return default
    if not isinstance(value, str):
        raise TypeError('"%s" must be a unicode character, not %s'
                        % (name, _type_name(value)))
    if len(value) != 1:
        raise TypeError('"%s" must be a unicode character, '
                        'not a string of length %d' % (name, len(value)))
    return value


def _set_char_or_none(name, value, default):
    if value is _NOT_SET:
        return default
    if value is None:
        return None
    if not isinstance(value, str):
        raise TypeError('"%s" must be a unicode character or None, not %s'
                        % (name, _type_name(value)))
    if len(value) != 1:
        raise TypeError('"%s" must be a unicode character or None, '
                        'not a string of length %d' % (name, len(value)))
    return value


def _set_bool(value, default):
    if value is _NOT_SET:
        return default
    return bool(value)


def _set_int(name, value, default):
    if value is _NOT_SET:
        return default
    if type(value) is not int:
        raise TypeError('"%s" must be an integer, not %s'
                        % (name, _type_name(value)))
    return value


def _set_str(name, value, default):
    if value is _NOT_SET:
        return default
    if not isinstance(value, str):
        raise TypeError('"%s" must be a string, not %s'
                        % (name, _type_name(value)))
    return value


def _getattr_or_missing(obj, name):
    try:
        return getattr(obj, name)
    except AttributeError:
        return _NOT_SET


def _check_char(name, c, lineterminator, allowspace):
    if c == '\r' or c == '\n' or (c == ' ' and not allowspace):
        raise ValueError('bad %s value' % name)
    if c is not None and c in lineterminator:
        raise ValueError('bad %s or lineterminator value' % name)


def _check_chars(name1, name2, c1, c2):
    if c1 == c2 and c1 is not None:
        raise ValueError('bad %s or %s value' % (name1, name2))


class Dialect:
    def __new__(cls, dialect=None, delimiter=_NOT_SET, doublequote=_NOT_SET,
                escapechar=_NOT_SET, lineterminator=_NOT_SET,
                quotechar=_NOT_SET, quoting=_NOT_SET,
                skipinitialspace=_NOT_SET, strict=_NOT_SET):
        if isinstance(dialect, str):
            if dialect not in _dialects:
                raise Error('unknown dialect')
            dialect = _dialects[dialect]
        if (isinstance(dialect, Dialect) and delimiter is _NOT_SET
                and doublequote is _NOT_SET and escapechar is _NOT_SET
                and lineterminator is _NOT_SET and quotechar is _NOT_SET
                and quoting is _NOT_SET and skipinitialspace is _NOT_SET
                and strict is _NOT_SET):
            return dialect
        return object.__new__(cls)

    def __init__(self, dialect=None, delimiter=_NOT_SET, doublequote=_NOT_SET,
                 escapechar=_NOT_SET, lineterminator=_NOT_SET,
                 quotechar=_NOT_SET, quoting=_NOT_SET,
                 skipinitialspace=_NOT_SET, strict=_NOT_SET):
        if dialect is not None:
            if isinstance(dialect, str):
                if dialect not in _dialects:
                    raise Error('unknown dialect')
                dialect = _dialects[dialect]
            if (dialect is self and delimiter is _NOT_SET
                    and doublequote is _NOT_SET and escapechar is _NOT_SET
                    and lineterminator is _NOT_SET and quotechar is _NOT_SET
                    and quoting is _NOT_SET and skipinitialspace is _NOT_SET
                    and strict is _NOT_SET):
                return
            if delimiter is _NOT_SET:
                delimiter = _getattr_or_missing(dialect, 'delimiter')
            if doublequote is _NOT_SET:
                doublequote = _getattr_or_missing(dialect, 'doublequote')
            if escapechar is _NOT_SET:
                escapechar = _getattr_or_missing(dialect, 'escapechar')
            if lineterminator is _NOT_SET:
                lineterminator = _getattr_or_missing(dialect, 'lineterminator')
            if quotechar is _NOT_SET:
                quotechar = _getattr_or_missing(dialect, 'quotechar')
            if quoting is _NOT_SET:
                quoting = _getattr_or_missing(dialect, 'quoting')
            if skipinitialspace is _NOT_SET:
                skipinitialspace = _getattr_or_missing(dialect, 'skipinitialspace')
            if strict is _NOT_SET:
                strict = _getattr_or_missing(dialect, 'strict')
        self._delimiter = _set_char('delimiter', delimiter, ',')
        self._doublequote = _set_bool(doublequote, True)
        self._escapechar = _set_char_or_none('escapechar', escapechar, None)
        self._lineterminator = _set_str('lineterminator', lineterminator, '\r\n')
        self._quotechar = _set_char_or_none('quotechar', quotechar, '"')
        self._quoting = _set_int('quoting', quoting, QUOTE_MINIMAL)
        self._skipinitialspace = _set_bool(skipinitialspace, False)
        self._strict = _set_bool(strict, False)
        if self._quoting not in _QUOTE_STYLES:
            raise TypeError('bad "quoting" value')
        if quotechar is None and quoting is _NOT_SET:
            self._quoting = QUOTE_NONE
        if self._quoting != QUOTE_NONE and self._quotechar is None:
            raise TypeError('quotechar must be set if quoting enabled')
        _check_char('delimiter', self._delimiter, self._lineterminator, True)
        _check_char('escapechar', self._escapechar, self._lineterminator,
                    not self._skipinitialspace)
        _check_char('quotechar', self._quotechar, self._lineterminator,
                    not self._skipinitialspace)
        _check_chars('delimiter', 'escapechar', self._delimiter, self._escapechar)
        _check_chars('delimiter', 'quotechar', self._delimiter, self._quotechar)
        _check_chars('escapechar', 'quotechar', self._escapechar, self._quotechar)

    @property
    def delimiter(self):
        return self._delimiter

    @property
    def doublequote(self):
        return self._doublequote

    @property
    def escapechar(self):
        return self._escapechar

    @property
    def lineterminator(self):
        return self._lineterminator

    @property
    def quotechar(self):
        return self._quotechar

    @property
    def quoting(self):
        return self._quoting

    @property
    def skipinitialspace(self):
        return self._skipinitialspace

    @property
    def strict(self):
        return self._strict

    def __reduce__(self, *args):
        raise TypeError("cannot pickle '_csv.Dialect' instances")

    __reduce_ex__ = __reduce__


def _call_dialect(dialect_inst, fmtparams):
    if dialect_inst is not None and isinstance(dialect_inst, str):
        if dialect_inst not in _dialects:
            raise Error('unknown dialect')
        dialect_inst = _dialects[dialect_inst]
    return Dialect(dialect_inst, **fmtparams)


class Reader:
    def __init__(self, input_iter, dialect):
        self._input_iter = input_iter
        self._dialect = dialect
        self._fields = None
        self._field = []
        self._field_length = 0
        self._state = START_RECORD
        self._unquoted = False
        self._line_num = 0

    @property
    def dialect(self):
        return self._dialect

    @property
    def line_num(self):
        return self._line_num

    def __iter__(self):
        return self

    def _save_field(self):
        quoting = self._dialect.quoting
        if self._unquoted and not self._field and \
                quoting in (QUOTE_NOTNULL, QUOTE_STRINGS):
            field = None
        else:
            field = ''.join(self._field)
            if self._unquoted and self._field and \
                    quoting in (QUOTE_NONNUMERIC, QUOTE_STRINGS):
                field = float(field)
            self._field = []
        self._field_length = 0
        self._fields.append(field)

    def _add_char(self, c):
        self._add_text(c)

    def _add_text(self, text):
        size = self._field_length + len(text)
        if size > _field_limit:
            raise Error('field larger than field limit (%d)' % _field_limit)
        self._field.append(text)
        self._field_length = size

    def _process_char(self, c):
        d = self._dialect
        state = self._state
        if state == START_RECORD:
            if c is _EOL:
                return
            if c == '\n' or c == '\r':
                self._state = EAT_CRNL
                return
            state = START_FIELD
            self._state = START_FIELD
        if state == START_FIELD:
            self._unquoted = True
            if c == '\n' or c == '\r' or c is _EOL:
                self._save_field()
                self._state = START_RECORD if c is _EOL else EAT_CRNL
            elif c == d.quotechar and d.quoting != QUOTE_NONE:
                self._unquoted = False
                self._state = IN_QUOTED_FIELD
            elif c == d.escapechar:
                self._state = ESCAPED_CHAR
            elif c == ' ' and d.skipinitialspace:
                pass
            elif c == d.delimiter:
                self._save_field()
            else:
                self._add_char(c)
                self._state = IN_FIELD
            return
        if state == ESCAPED_CHAR:
            if c == '\n' or c == '\r':
                self._add_char(c)
                self._state = AFTER_ESCAPED_CRNL
            else:
                if c is _EOL:
                    c = '\n'
                self._add_char(c)
                self._state = IN_FIELD
            return
        if state == AFTER_ESCAPED_CRNL:
            if c is _EOL:
                return
            state = IN_FIELD
            self._state = IN_FIELD
        if state == IN_FIELD:
            if c == '\n' or c == '\r' or c is _EOL:
                self._save_field()
                self._state = START_RECORD if c is _EOL else EAT_CRNL
            elif c == d.escapechar:
                self._state = ESCAPED_CHAR
            elif c == d.delimiter:
                self._save_field()
                self._state = START_FIELD
            else:
                self._add_char(c)
            return
        if state == IN_QUOTED_FIELD:
            if c is _EOL:
                pass
            elif c == d.escapechar:
                self._state = ESCAPE_IN_QUOTED_FIELD
            elif c == d.quotechar and d.quoting != QUOTE_NONE:
                if d.doublequote:
                    self._state = QUOTE_IN_QUOTED_FIELD
                else:
                    self._state = IN_FIELD
            else:
                self._add_char(c)
            return
        if state == ESCAPE_IN_QUOTED_FIELD:
            if c is _EOL:
                c = '\n'
            self._add_char(c)
            self._state = IN_QUOTED_FIELD
            return
        if state == QUOTE_IN_QUOTED_FIELD:
            if d.quoting != QUOTE_NONE and c == d.quotechar:
                self._add_char(c)
                self._state = IN_QUOTED_FIELD
            elif c == d.delimiter:
                self._save_field()
                self._state = START_FIELD
            elif c == '\n' or c == '\r' or c is _EOL:
                self._save_field()
                self._state = START_RECORD if c is _EOL else EAT_CRNL
            elif not d.strict:
                self._add_char(c)
                self._state = IN_FIELD
            else:
                raise Error("'%s' expected after '%s'"
                            % (d.delimiter, d.quotechar))
            return
        if state == EAT_CRNL:
            if c == '\n' or c == '\r':
                pass
            elif c is _EOL:
                self._state = START_RECORD
            else:
                raise Error("new-line character seen in unquoted field - "
                            "do you need to open the file with newline=''?")
            return

    def __next__(self):
        self._fields = []
        self._field = []
        self._field_length = 0
        self._state = START_RECORD
        self._unquoted = False
        while True:
            try:
                lineobj = next(self._input_iter)
            except StopIteration:
                if self._field or self._state == IN_QUOTED_FIELD:
                    if self._dialect.strict:
                        raise Error('unexpected end of data')
                    self._save_field()
                    break
                raise
            if not isinstance(lineobj, str):
                raise Error('iterator should return strings, not %s '
                            '(the file should be opened in text mode)'
                            % _type_name(lineobj))
            if self._fields is None:
                raise Error('iterator has already advanced the reader')
            self._line_num += 1
            if type(lineobj) is str:
                position = 0
                length = len(lineobj)
                while position < length:
                    if self._state in (IN_FIELD, IN_QUOTED_FIELD):
                        d = self._dialect
                        if self._state == IN_FIELD:
                            stops = (d.delimiter, d.escapechar, '\r', '\n')
                        else:
                            stops = (d.escapechar, d.quotechar if d.quoting != QUOTE_NONE else None)
                        end = length
                        for stop in stops:
                            if stop is not None:
                                found = lineobj.find(stop, position)
                                if found >= 0 and found < end:
                                    end = found
                        if end > position:
                            self._add_text(lineobj[position:end])
                            position = end
                            continue
                    self._process_char(lineobj[position])
                    position += 1
            else:
                for c in lineobj:
                    self._process_char(c)
            self._process_char(_EOL)
            if self._state == START_RECORD:
                break
        fields = self._fields
        self._fields = None
        return fields


class Writer:
    def __init__(self, write, dialect):
        self._write = write
        self._dialect = dialect
        self._rec = []
        self._num_fields = 0

    @property
    def dialect(self):
        return self._dialect

    def _join_append(self, field, quoted):
        d = self._dialect
        chars = '' if field is None else field
        if not chars and d.delimiter == ' ' and d.skipinitialspace:
            if d.quoting == QUOTE_NONE or \
                    (field is None and d.quoting in (QUOTE_STRINGS, QUOTE_NOTNULL)):
                raise Error('empty field must be quoted if delimiter is a '
                            'space and skipinitialspace is true')
            quoted = True
        if type(chars) is str:
            replacements = {}
            for c in (d.delimiter, d.escapechar, d.quotechar, '\n', '\r') + tuple(d.lineterminator):
                if c is None or ord(c) in replacements or chars.find(c) < 0:
                    continue
                if d.quoting == QUOTE_NONE:
                    want_escape = True
                elif c == d.quotechar:
                    want_escape = not d.doublequote
                    if d.doublequote:
                        quoted = True
                elif c == d.escapechar:
                    want_escape = True
                else:
                    want_escape = False
                    quoted = True
                if want_escape:
                    if d.escapechar is None:
                        raise Error('need to escape, but no escapechar set')
                    replacements[ord(c)] = d.escapechar + c
                elif c == d.quotechar and d.doublequote:
                    replacements[ord(c)] = c + c
                else:
                    replacements[ord(c)] = c
            if self._num_fields:
                self._rec.append(d.delimiter)
            if quoted:
                self._rec.append(d.quotechar)
            self._rec.append(chars.translate(replacements) if replacements else chars)
            if quoted:
                self._rec.append(d.quotechar)
            self._num_fields += 1
            return
        for c in chars:
            if c == d.delimiter or c == d.escapechar or c == d.quotechar or \
                    c == '\n' or c == '\r' or c in d.lineterminator:
                if d.quoting == QUOTE_NONE:
                    want_escape = True
                else:
                    if c == d.quotechar:
                        want_escape = not d.doublequote
                    elif c == d.escapechar:
                        want_escape = True
                    else:
                        want_escape = False
                    if not want_escape:
                        quoted = True
                if want_escape and d.escapechar is None:
                    raise Error('need to escape, but no escapechar set')
        if self._num_fields > 0:
            self._rec.append(d.delimiter)
        if quoted:
            self._rec.append(d.quotechar)
        for c in chars:
            if c == d.delimiter or c == d.escapechar or c == d.quotechar or \
                    c == '\n' or c == '\r' or c in d.lineterminator:
                if d.quoting == QUOTE_NONE:
                    want_escape = True
                elif c == d.quotechar:
                    if d.doublequote:
                        self._rec.append(d.quotechar)
                        want_escape = False
                    else:
                        want_escape = True
                elif c == d.escapechar:
                    want_escape = True
                else:
                    want_escape = False
                if want_escape:
                    self._rec.append(d.escapechar)
            self._rec.append(c)
        if quoted:
            self._rec.append(d.quotechar)
        self._num_fields += 1

    def writerow(self, seq):
        d = self._dialect
        try:
            it = iter(seq)
        except TypeError:
            raise Error('iterable expected, not %s' % _type_name(seq))
        self._num_fields = 0
        self._rec = []
        null_field = False
        for field in it:
            if d.quoting == QUOTE_NONNUMERIC:
                quoted = not _is_number(field)
            elif d.quoting == QUOTE_ALL:
                quoted = True
            elif d.quoting == QUOTE_STRINGS:
                quoted = isinstance(field, str)
            elif d.quoting == QUOTE_NOTNULL:
                quoted = field is not None
            else:
                quoted = False
            null_field = field is None
            if isinstance(field, str):
                self._join_append(field, quoted)
            elif field is None:
                self._join_append(None, quoted)
            else:
                self._join_append(str(field), quoted)
        if self._num_fields > 0 and not self._rec:
            if d.quoting == QUOTE_NONE or \
                    (null_field and d.quoting in (QUOTE_STRINGS, QUOTE_NOTNULL)):
                raise Error('single empty field record must be quoted')
            self._num_fields -= 1
            self._join_append(None, True)
        self._rec.append(d.lineterminator)
        line = ''.join(self._rec)
        return self._write(line)

    def writerows(self, rows):
        for row in rows:
            self.writerow(row)
        return None


def _is_number(value):
    if isinstance(value, complex):
        return True
    t = type(value)
    return (hasattr(t, '__index__') or hasattr(t, '__int__') or
            hasattr(t, '__float__'))


def reader(iterator, dialect=None, **fmtparams):
    input_iter = iter(iterator)
    return Reader(input_iter, _call_dialect(dialect, fmtparams))


def writer(output_file, dialect=None, **fmtparams):
    write = getattr(output_file, 'write', None)
    if write is None or not callable(write):
        raise TypeError('argument 1 must have a "write" method')
    return Writer(write, _call_dialect(dialect, fmtparams))


def register_dialect(name, dialect=None, **fmtparams):
    if not isinstance(name, str):
        raise TypeError('dialect name must be a string')
    _dialects[name] = _call_dialect(dialect, fmtparams)


def unregister_dialect(name):
    if name not in _dialects:
        raise Error('unknown dialect')
    del _dialects[name]


def get_dialect(name):
    if name not in _dialects:
        raise Error('unknown dialect')
    return _dialects[name]


def list_dialects():
    return list(_dialects)


def field_size_limit(new_limit=_NOT_SET):
    global _field_limit
    old = _field_limit
    if new_limit is not _NOT_SET:
        if type(new_limit) is not int:
            raise TypeError('limit must be an integer')
        _field_limit = new_limit
    return old
