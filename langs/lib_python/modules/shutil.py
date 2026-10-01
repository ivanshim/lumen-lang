import os

class terminal_size(tuple):
    # A width and a height, as a tuple of the two.
    def __new__(cls, size):
        return tuple.__new__(cls, size)

    @property
    def columns(self):
        return self[0]

    @property
    def lines(self):
        return self[1]

def get_terminal_size(fallback=(80, 24)):
    # The terminal's width and height, as the environment says they
    # are, or the fallback when it says nothing. No terminal is asked
    # directly: none stands behind this library's streams.
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
    return terminal_size((columns, lines))

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
