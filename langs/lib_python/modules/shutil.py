import os

def rmtree(path, ignore_errors=False, onerror=None, *, onexc=None, dir_fd=None):
    if __remove_tree(path):
        return
    if ignore_errors:
        return
    if onexc is not None or onerror is not None:
        handler = onexc if onexc is not None else onerror
        try:
            os.listdir(path)
        except Exception as error:
            handler(rmtree, path, error)
        return
    if not os.path.exists(path):
        raise FileNotFoundError(2, 'No such file or directory', path)
    raise OSError('rmtree failed for ' + path)

import sys

# Source: CPython 3b564385e4c9, Lib/shutil.py; PSF License.
def get_terminal_size(fallback=(80, 24)):
    """Get the size of the terminal window.

    For each of the two dimensions, the environment variable, COLUMNS
    and LINES respectively, is checked. If the variable is defined and
    the value is a positive integer, it is used.

    When COLUMNS or LINES is not defined, which is the common case,
    the terminal connected to sys.__stdout__ is queried
    by invoking os.get_terminal_size.

    If the terminal size cannot be successfully queried, either because
    the system doesn't support querying, or because we are not
    connected to a terminal, the value given in fallback parameter
    is used. Fallback defaults to (80, 24) which is the default
    size used by many terminal emulators.

    The value returned is a named tuple of type os.terminal_size.
    """
    # columns, lines are the working values
    try:
        columns = int(os.environ['COLUMNS'])
    except (KeyError, ValueError):
        columns = 0

    try:
        lines = int(os.environ['LINES'])
    except (KeyError, ValueError):
        lines = 0

    # only query if necessary
    if columns <= 0 or lines <= 0:
        try:
            size = os.get_terminal_size(sys.__stdout__.fileno())
        except (AttributeError, ValueError, OSError):
            # stdout is None, closed, detached, or not a terminal, or
            # os.get_terminal_size() is unsupported
            size = os.terminal_size(fallback)
        if columns <= 0:
            columns = size.columns or fallback[0]
        if lines <= 0:
            lines = size.lines or fallback[1]

    return os.terminal_size((columns, lines))
