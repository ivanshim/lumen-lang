def format_tb(tb, limit=None, exc=None):
    frames = []
    while tb is not None:
        frames.append(tb)
        tb = tb.tb_next
    if limit is not None:
        if limit >= 0:
            frames = frames[:limit]
        else:
            frames = frames[limit:]
    result = []
    for item in frames:
        code = item.tb_frame.f_code
        filename = code.co_filename
        line = item.tb_lineno
        entry = '  File "' + filename + '", line ' + str(line) + ', in ' + code.co_name + '\n'
        try:
            import linecache
            lines = linecache.getlines(filename, item.tb_frame.f_globals)
            shown = lines[line - 1].strip()
            mark = None
            if isinstance(exc, AssertionError) and shown.startswith('assert '):
                expression = shown[7:].split(',', 1)[0]
                if expression == '(' and line < len(lines):
                    following = lines[line].strip()
                    shown = following
                    if following.endswith('and') and line + 1 < len(lines):
                        shown += '\n    ' + lines[line + 1].strip()
                    elif ')' in following:
                        mark = following.index(')')
                else:
                    mark = len(expression)
            if shown:
                entry += '    ' + shown + '\n'
                if mark is not None and mark > 0:
                    entry += '    ' + ' ' * (7 if shown.startswith('assert ') else 0) + '^' * mark + '\n'
                elif mark is None:
                    carets = _frame_carets(item, lines[line - 1])
                    if carets is not None:
                        entry += '    ' + carets + '\n'
        except (OSError, IndexError, UnicodeError):
            pass
        result.append(entry)
    return result


def format_exception_only(exc, value=None):
    if value is not None:
        exc = value
    try:
        kind, message = __current_fault(exc)
    except BaseException:
        kind = type(exc).__name__
        message = '<exception str() failed>'
    if kind is None:
        raise 'TypeError: an exception value is required'
    if isinstance(exc, BaseException):
        exception_type = type(exc)
        kind = exception_type.__qualname__
        module = exception_type.__module__
        if module not in ('builtins', '__main__'):
            kind = module + '.' + kind
    prefix = kind + ': '
    if message[:len(prefix)] == prefix:
        message = message[len(prefix):]
    result = []
    if isinstance(exc, SyntaxError):
        suffix = ''
        if exc.lineno is not None:
            result.append('  File "' + (exc.filename or '<string>') + '", line ' + str(exc.lineno) + '\n')
        elif exc.filename is not None:
            suffix = ' (' + exc.filename + ')'
        if exc.text is not None:
            original = exc.text.rstrip('\n')
            shown = original.lstrip(' \n\f')
            removed = len(original) - len(shown)
            result.append('    ' + shown + '\n')
            if isinstance(exc.offset, int):
                offset = exc.offset
                if exc.lineno == exc.end_lineno:
                    end = exc.end_offset
                    if not isinstance(end, int) or end == 0:
                        end = offset
                else:
                    end = len(original) + 1
                if exc.text and offset > len(exc.text):
                    offset = len(original) + 1
                if exc.text and end > len(exc.text):
                    end = len(original) + 1
                if offset >= end or end < 0:
                    end = offset + 1
                start = offset - 1 - removed
                if start >= 0:
                    padding = ''
                    for ch in shown[:start]:
                        padding += ch if ch.isspace() else ' '
                    result.append('    ' + padding + '^' * (end - offset) + '\n')
        message = str(exc.msg or '<no detail available>') + suffix
    cls = type(exc)
    module = cls.__module__
    display = cls.__qualname__
    if module not in ('builtins', '__main__'):
        display = module + '.' + display
    result.append((display + ': ' + message if message else display) + '\n')
    notes = getattr(exc, '__notes__', None)
    if isinstance(notes, (list, tuple)):
        for note in notes:
            for line in str(note).split('\n'):
                result.append(line + '\n')
    return result


def _format_exception(exc, tb, limit, chain, seen):
    if id(exc) in seen:
        return []
    seen.add(id(exc))
    result = []
    if chain:
        cause = exc.__cause__
        context = exc.__context__
        if cause is not None:
            result.extend(_format_exception(cause, cause.__traceback__, limit, chain, seen))
            result.append('\nThe above exception was the direct cause of the following exception:\n\n')
        elif context is not None and not exc.__suppress_context__:
            result.extend(_format_exception(context, context.__traceback__, limit, chain, seen))
            result.append('\nDuring handling of the above exception, another exception occurred:\n\n')
    if tb is not None:
        frames = format_tb(tb, limit, exc)
        if frames:
            result.append('Traceback (most recent call last):\n')
            result.extend(frames)
    result.extend(format_exception_only(exc))
    return result


def format_exception(exc, value=None, tb=None, limit=None, chain=True):
    if value is not None:
        exc = value
    else:
        tb = exc.__traceback__
    return _format_exception(exc, tb, limit, chain, set())


def format_exc(limit=None, chain=True):
    held = __fault_in_hand()
    if held is None:
        return 'NoneType: None\n'
    return ''.join(format_exception(held, limit=limit, chain=chain))


def print_exception(exc, value=None, tb=None, limit=None, file=None, chain=True):
    import sys
    if file is None:
        file = sys.stderr
    file.write(''.join(format_exception(exc, value, tb, limit, chain)))


def print_exc(limit=None, file=None, chain=True):
    import sys
    if file is None:
        file = sys.stderr
    file.write(format_exc(limit, chain))


class FrameSummary:
    def __init__(self, filename, lineno, name):
        self.filename = filename
        self.lineno = lineno
        self.name = name
        self.end_lineno = None
        self.colno = None
        self.end_colno = None
        self.locals = None
        self._line = None

    @property
    def line(self):
        if self._line is None:
            self._line = ''
            try:
                import builtins
                with builtins.open(self.filename) as source:
                    self._line = source.read().splitlines()[self.lineno - 1].strip()
            except (OSError, IndexError):
                pass
        return self._line.strip()

    def __iter__(self):
        return iter((self.filename, self.lineno, self.name, self.line))

    def __getitem__(self, index):
        return (self.filename, self.lineno, self.name, self.line)[index]

    def __len__(self):
        return 4


class StackSummary:
    def __init__(self, frames):
        self._frames = list(frames)

    def __iter__(self):
        return iter(self._frames)

    def __len__(self):
        return len(self._frames)

    def __getitem__(self, index):
        return self._frames[index]

    def append(self, frame):
        self._frames.append(frame)

    def reverse(self):
        self._frames.reverse()

    def format(self):
        result = []
        for frame in self:
            entry = '  File "' + frame.filename + '", line ' + str(frame.lineno) + ', in ' + frame.name + '\n'
            if frame.line:
                entry += '    ' + frame.line + '\n'
            result.append(entry)
        return result


def _limited_frames(frames, limit):
    if limit is None:
        import sys
        limit = getattr(sys, 'tracebacklimit', None)
        if limit is not None and limit < 0:
            limit = 0
    if limit is not None:
        frames = frames[:limit] if limit >= 0 else frames[limit:]
    return StackSummary(frames)


def extract_tb(tb, limit=None):
    frames = []
    while tb is not None:
        code = tb.tb_frame.f_code
        frame = FrameSummary(code.co_filename, tb.tb_lineno, code.co_name)
        frame.end_lineno = tb.tb_end_lineno
        frame.colno = tb.tb_colno
        frame.end_colno = tb.tb_end_colno
        frames.append(frame)
        tb = tb.tb_next
    return _limited_frames(frames, limit)


def extract_stack(f=None, limit=None):
    if f is None:
        import sys
        f = sys._getframe(1)
    frames = []
    while f is not None:
        frames.append(FrameSummary(f.f_code.co_filename, f.f_lineno, f.f_code.co_name))
        f = f.f_back
    frames = _limited_frames(frames, limit)
    frames.reverse()
    return frames


def format_list(extracted_list):
    frames = StackSummary([])
    for frame in extracted_list:
        if not isinstance(frame, FrameSummary):
            filename, lineno, name, line = frame
            frame = FrameSummary(filename, lineno, name)
            frame._line = line
        frames.append(frame)
    return frames.format()


def format_stack(f=None, limit=None):
    if f is None:
        import sys
        f = sys._getframe(1)
    return extract_stack(f, limit).format()


class TracebackException:
    # Capture traceback frames with the standard constructor options.
    def __init__(self, exc_type, exc_value, exc_traceback, *, limit=None,
                 lookup_lines=True, capture_locals=False, compact=False):
        if capture_locals:
            raise NotImplementedError('TracebackException local-variable capture is unavailable')
        self.exc_type = exc_type
        self._limit = limit
        self.stack = extract_tb(exc_traceback, self._limit)
        if lookup_lines:
            for frame in self.stack:
                frame.line
        self._exception = exc_value
        self._traceback = exc_traceback

    @classmethod
    def from_exception(cls, exc, *args, **kwargs):
        return cls(type(exc), exc, exc.__traceback__, *args, **kwargs)

    # Format a plain traceback; refuse unsupported terminal color rendering.
    def format(self, *, chain=True, colorize=False):
        if colorize:
            raise NotImplementedError('TracebackException color rendering is unavailable')
        return iter(format_exception(self.exc_type, self._exception, self._traceback,
                                     limit=self._limit, chain=chain))

    def format_exception_only(self):
        return iter(format_exception_only(self._exception))

    def print(self, *, file=None, chain=True):
        import sys
        if file is None:
            file = sys.stderr
        file.write(''.join(self.format(chain=chain)))


import collections
from contextlib import suppress

def _byte_offset_to_character_offset(str, offset):
    as_utf8 = str.encode('utf-8')
    return len(as_utf8[:offset].decode("utf-8", errors="replace"))


_Anchors = collections.namedtuple(
    "_Anchors",
    [
        "left_end_lineno",
        "left_end_offset",
        "right_start_lineno",
        "right_start_offset",
        "primary_char",
        "secondary_char",
    ],
    defaults=["~", "^"]
)

def _extract_caret_anchors_from_line_segment(segment):
    """
    Given source code `segment` corresponding to a FrameSummary, determine:
        - for binary ops, the location of the binary op
        - for indexing and function calls, the location of the brackets.
    `segment` is expected to be a valid Python expression.
    """
    import ast

    try:
        # Without parentheses, `segment` is parsed as a statement.
        # Binary ops, subscripts, and calls are expressions, so
        # we can wrap them with parentheses to parse them as
        # (possibly multi-line) expressions.
        # e.g. if we try to highlight the addition in
        # x = (
        #     a +
        #     b
        # )
        # then we would ast.parse
        #     a +
        #     b
        # which is not a valid statement because of the newline.
        # Adding brackets makes it a valid expression.
        # (
        #     a +
        #     b
        # )
        # Line locations will be different than the original,
        # which is taken into account later on.
        tree = ast.parse(f"(\n{segment}\n)")
    except SyntaxError:
        return None

    if len(tree.body) != 1:
        return None

    lines = segment.splitlines()

    def normalize(lineno, offset):
        """Get character index given byte offset"""
        return _byte_offset_to_character_offset(lines[lineno], offset)

    def next_valid_char(lineno, col):
        """Gets the next valid character index in `lines`, if
        the current location is not valid. Handles empty lines.
        """
        while lineno < len(lines) and col >= len(lines[lineno]):
            col = 0
            lineno += 1
        assert lineno < len(lines) and col < len(lines[lineno])
        return lineno, col

    def increment(lineno, col):
        """Get the next valid character index in `lines`."""
        col += 1
        lineno, col = next_valid_char(lineno, col)
        return lineno, col

    def nextline(lineno, col):
        """Get the next valid character at least on the next line"""
        col = 0
        lineno += 1
        lineno, col = next_valid_char(lineno, col)
        return lineno, col

    def increment_until(lineno, col, stop):
        """Get the next valid non-"\\#" character that satisfies the `stop` predicate"""
        while True:
            ch = lines[lineno][col]
            if ch in "\\#":
                lineno, col = nextline(lineno, col)
            elif not stop(ch):
                lineno, col = increment(lineno, col)
            else:
                break
        return lineno, col

    def setup_positions(expr, force_valid=True):
        """Get the lineno/col position of the end of `expr`. If `force_valid` is True,
        forces the position to be a valid character (e.g. if the position is beyond the
        end of the line, move to the next line)
        """
        # -2 since end_lineno is 1-indexed and because we added an extra
        # bracket + newline to `segment` when calling ast.parse
        lineno = expr.end_lineno - 2
        col = normalize(lineno, expr.end_col_offset)
        return next_valid_char(lineno, col) if force_valid else (lineno, col)

    statement = tree.body[0]
    match statement:
        case ast.Expr(value=expr):
            match expr:
                case ast.BinOp():
                    # ast gives these locations for BinOp subexpressions
                    # ( left_expr ) + ( right_expr )
                    #   left^^^^^       right^^^^^
                    lineno, col = setup_positions(expr.left)

                    # First operator character is the first non-space/')' character
                    lineno, col = increment_until(lineno, col, lambda x: not x.isspace() and x != ')')

                    # binary op is 1 or 2 characters long, on the same line,
                    # before the right subexpression
                    right_col = col + 1
                    if (
                        right_col < len(lines[lineno])
                        and (
                            # operator char should not be in the right subexpression
                            expr.right.lineno - 2 > lineno or
                            right_col < normalize(expr.right.lineno - 2, expr.right.col_offset)
                        )
                        and not (ch := lines[lineno][right_col]).isspace()
                        and ch not in "\\#"
                    ):
                        right_col += 1

                    # right_col can be invalid since it is exclusive
                    return _Anchors(lineno, col, lineno, right_col)
                case ast.Subscript():
                    # ast gives these locations for value and slice subexpressions
                    # ( value_expr ) [ slice_expr ]
                    #   value^^^^^     slice^^^^^
                    # subscript^^^^^^^^^^^^^^^^^^^^

                    # find left bracket
                    left_lineno, left_col = setup_positions(expr.value)
                    left_lineno, left_col = increment_until(left_lineno, left_col, lambda x: x == '[')
                    # find right bracket (final character of expression)
                    right_lineno, right_col = setup_positions(expr, force_valid=False)
                    return _Anchors(left_lineno, left_col, right_lineno, right_col)
                case ast.Call():
                    # ast gives these locations for function call expressions
                    # ( func_expr ) (args, kwargs)
                    #   func^^^^^
                    # call^^^^^^^^^^^^^^^^^^^^^^^^

                    # find left bracket
                    left_lineno, left_col = setup_positions(expr.func)
                    left_lineno, left_col = increment_until(left_lineno, left_col, lambda x: x == '(')
                    # find right bracket (final character of expression)
                    right_lineno, right_col = setup_positions(expr, force_valid=False)
                    return _Anchors(left_lineno, left_col, right_lineno, right_col)

    return None

_WIDE_CHAR_SPECIFIERS = "WF"

def _display_width(line, offset=None):
    """Calculate the amount of width space the given source
    code segment might take if it were to be displayed on a fixed
    width output device. Supports wide unicode characters and emojis."""

    if offset is None:
        offset = len(line)

    # Fast track for ASCII-only strings
    if line.isascii():
        return offset

    import unicodedata

    return sum(
        2 if unicodedata.east_asian_width(char) in _WIDE_CHAR_SPECIFIERS else 1
        for char in line[:offset]
    )




def _should_show_carets(start_offset, end_offset, all_lines, anchors):
    with suppress(SyntaxError, ImportError):
        import ast
        tree = ast.parse('\n'.join(all_lines))
        if not tree.body:
            return False
        statement = tree.body[0]
        value = None
        def _spawns_full_line(value):
            return (
                value.lineno == 1
                and value.end_lineno == len(all_lines)
                and value.col_offset == start_offset
                and value.end_col_offset == end_offset
            )
        if isinstance(statement, getattr(ast, 'Return', ())):
            if isinstance(statement.value, ast.Call) and isinstance(statement.value.func, ast.Name):
                value = statement.value
        elif isinstance(statement, getattr(ast, 'Assign', ())):
            if (isinstance(statement.value, ast.Call) and len(statement.targets) == 1
                    and isinstance(statement.targets[0], ast.Name)):
                value = statement.value
        if value is not None and _spawns_full_line(value):
            return False
    if anchors:
        return True
    if all_lines[0][:start_offset].lstrip() or all_lines[-1][end_offset:].rstrip():
        return True
    return False



def _frame_carets(frame, original):
    # shortcut: frame carets cover one source line; use FrameSummary for multi-line spans.
    try:
        if frame.tb_end_lineno != frame.tb_lineno:
            return None
        if frame.tb_colno is None or frame.tb_end_colno is None:
            return None
        original = original.rstrip('\r\n')
        start = _byte_offset_to_character_offset(original, frame.tb_colno)
        end = _byte_offset_to_character_offset(original, frame.tb_end_colno)
        removed = len(original) - len(original.lstrip())
        shown = original.lstrip()
        start = max(0, start - removed)
        end = max(0, end - removed)
        segment = shown[start:end]
        anchors = _extract_caret_anchors_from_line_segment(segment)
        if not _should_show_carets(start, end, [shown], anchors):
            return None
        left = right = 0
        if anchors:
            left = _display_width(shown, start + anchors.left_end_offset)
            right = _display_width(shown, start + anchors.right_start_offset)
        first = _display_width(shown, start)
        last = _display_width(shown, end)
        characters = []
        for column in range(last):
            if column < first:
                characters.append(' ')
            elif anchors and left <= column < right:
                characters.append(anchors.secondary_char)
            else:
                characters.append(anchors.primary_char if anchors else '^')
        return ''.join(characters)
    except Exception:
        return None
# Release traceback locals; CPython v3.14.8 Lib/traceback.py, PSF License.
def clear_frames(tb):
    "Clear all references to local variables in the frames of a traceback."
    while tb is not None:
        try:
            tb.tb_frame.clear()
        except RuntimeError:
            # Ignore the exception raised if the frame is still executing.
            pass
        tb = tb.tb_next
