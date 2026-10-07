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
            import builtins
            with builtins.open(filename) as source:
                contents = source.read()
            if contents[:1] == '\ufeff':
                contents = contents[1:]
            lines = contents.splitlines()
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
            if exc.offset is not None:
                start = exc.offset - 1 - removed
                end = exc.end_offset
                if end is None or end == 0:
                    end = exc.offset
                if end == exc.offset or end == -1:
                    end = exc.offset + 1
                if start >= 0:
                    padding = ''
                    for ch in shown[:start]:
                        padding += ch if ch.isspace() else ' '
                    result.append('    ' + padding + '^' * max(1, min(end - exc.offset, len(shown) - start)) + '\n')
        message = str(exc.msg or '<no detail available>') + suffix
    result.append((prefix + message if message else kind) + '\n')
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
