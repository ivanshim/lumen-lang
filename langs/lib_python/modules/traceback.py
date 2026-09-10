# Only the last line is carried; frames await traceback values.
def format_exception(exc, value=None, tb=None, limit=None, chain=True):
    if value is not None:
        exc = value
    kind, message = __current_fault(exc)
    if kind is None:
        raise 'TypeError: an exception value is required'
    prefix = kind + ': '
    if message[:len(prefix)] == prefix:
        message = message[len(prefix):]
    return [prefix + message + '\n']

def format_exc(limit=None, chain=True):
    kind, message = __current_fault()
    if kind is None:
        if message is None:
            return 'NoneType: None\n'
        return str(message) + '\n'
    prefix = kind + ': '
    if message[:len(prefix)] == prefix:
        message = message[len(prefix):]
    return prefix + message + '\n'


def print_exc(limit=None, file=None, chain=True):
    import sys
    if file is None:
        file = sys.stderr
    file.write(format_exc(limit, chain))
