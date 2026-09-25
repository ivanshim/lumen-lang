def format_tb(tb, limit=None):
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
                text = source.read().splitlines()[line - 1].strip()
            if text:
                entry += '    ' + text + '\n'
        except (OSError, IndexError):
            pass
        result.append(entry)
    return result


def format_exception_only(exc, value=None):
    if value is not None:
        exc = value
    kind, message = __current_fault(exc)
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
                    result.append('    ' + padding + '^' * (end - exc.offset) + '\n')
        message = str(exc.msg or '<no detail available>') + suffix
    result.append(prefix + message + '\n')
    notes = getattr(exc, '__notes__', None)
    if isinstance(notes, (list, tuple)):
        for note in notes:
            result.append(str(note) + '\n')
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
        result.append('Traceback (most recent call last):\n')
        result.extend(format_tb(tb, limit))
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
