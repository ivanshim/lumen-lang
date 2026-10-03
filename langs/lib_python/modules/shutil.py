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

def get_terminal_size(fallback=(80, 24)):
    """Get the size of the terminal window.

    For each of the two dimensions, the environment variable, COLUMNS
    or LINES, is used first. If either is undefined or not a number, the
    fallback is used, this runtime carrying no terminal of its own to
    ask.
    """
    try:
        columns = int(os.environ['COLUMNS'])
    except (KeyError, ValueError):
        columns = 0
    try:
        lines = int(os.environ['LINES'])
    except (KeyError, ValueError):
        lines = 0
    if columns <= 0:
        columns = fallback[0]
    if lines <= 0:
        lines = fallback[1]
    return os.terminal_size((columns, lines))
