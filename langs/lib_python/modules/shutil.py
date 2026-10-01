import os
import sys

# Selected source from CPython commit 3b564385e4c9, Lib/shutil.py.
# Copyright (c) 2001 Python Software Foundation; All Rights Reserved.
# Used under the PSF license in tests/python/LICENSE.
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


# Check that a given file can be accessed with the correct mode.
# Additionally check that `file` is not a directory, as on Windows
# directories pass the os.access check.
def _access_check(fn, mode):
    return (os.path.exists(fn) and os.access(fn, mode)
            and not os.path.isdir(fn))


def _win_path_needs_curdir(cmd, mode):
    """
    On Windows, we can use NeedCurrentDirectoryForExePath to figure out
    if we should add the cwd to PATH when searching for executables if
    the mode is executable.
    """
    return (not (mode & os.X_OK)) or _winapi.NeedCurrentDirectoryForExePath(
                os.fsdecode(cmd))


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
