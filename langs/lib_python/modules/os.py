# Host primitives used by class bodies need non-private module bindings.
_host_file_exists = __file_exists
_host_file_kind = __file_kind

# The path rules below are POSIX rules. Host facts are read afresh.
name = 'posix'
sep = '/'
extsep = '.'
altsep = None
linesep = '\n'
curdir = '.'
pardir = '..'
pathsep = ':'
environ = __host_info()[3]

def getenv(key, default=None):
    return environ.get(key, default)

def getcwd():
    directory = __host_info()[0]
    if directory is None:
        raise 'OSError: working directory is unavailable'
    return directory

def chdir(path):
    if not __change_dir(path):
        if not _host_file_exists(path):
            raise FileNotFoundError(2, 'No such file or directory', path)
        raise OSError(20, 'Not a directory', path)

def getpid():
    # Stub: no real process identity is promised.
    return 1

def urandom(size):
    # Stub: byte storage is not yet available; no text masquerades as bytes.
    raise 'NotImplementedError: os.urandom needs byte values'

def listdir(path='.'):
    if _host_file_kind(path) == 1:
        raise OSError(20, 'Not a directory', path)
    entries = __list_dir(path)
    if entries is False:
        raise FileNotFoundError(2, 'No such file or directory', path)
    return entries

def mkdir(path, mode=511, *, dir_fd=None):
    if dir_fd is not None:
        raise 'NotImplementedError: os.mkdir directory descriptors are not supported'
    if not __make_dir_one(path):
        if _host_file_exists(path):
            raise OSError(17, 'File exists', path)
        raise FileNotFoundError(2, 'No such file or directory', path)

def remove(path, *, dir_fd=None):
    if dir_fd is not None:
        raise 'NotImplementedError: os.remove directory descriptors are not supported'
    if not __remove_file(path):
        if not __file_exists(path):
            raise FileNotFoundError(2, 'No such file or directory', path)
        raise OSError(1, 'Operation not permitted', path)

unlink = remove

class _Path:
    def join(self, path, *parts):
        for part in parts:
            if part[:1] == '/':
                path = part
            elif path == '' or path[-1:] == '/':
                path += part
            else:
                path += '/' + part
        return path

    def split(self, path):
        at = len(path)
        while at > 0 and path[at - 1] != '/':
            at -= 1
        head = path[:at]
        named = False
        for letter in list(head):
            if letter != '/':
                named = True
        if named:
            while head[-1:] == '/':
                head = head[:-1]
        return (head, path[at:])

    def basename(self, path):
        return self.split(path)[1]

    def dirname(self, path):
        return self.split(path)[0]

    def splitext(self, path):
        at = len(path) - 1
        while at >= 0 and path[at] != '/':
            if path[at] == '.':
                start = at - 1
                while start >= 0 and path[start] != '/':
                    if path[start] != '.':
                        return (path[:at], path[at:])
                    start -= 1
                break
            at -= 1
        return (path, '')

    def exists(self, path):
        return _host_file_exists(path)

    def isfile(self, path):
        return _host_file_kind(path) == 1

    def isdir(self, path):
        return _host_file_kind(path) == 2

    def abspath(self, path):
        if path[:1] != '/':
            path = self.join(getcwd(), path)
        parts = []
        words = []
        word = ''
        for letter in list(path + '/'):
            if letter == '/':
                words.append(word)
                word = ''
            else:
                word += letter
        for part in words:
            if part == '..':
                if len(parts) > 0:
                    parts = parts[:-1]
            elif part != '' and part != '.':
                parts.append(part)
        answer = ''
        for part in parts:
            answer += '/' + part
        return answer if answer else '/'

    def realpath(self, path):
        # The host gives no way to follow a link, so the answer is
        # the path made absolute and tidy, which is the whole of it
        # for a file a run has made itself.
        return self.abspath(path)

    def normcase(self, path):
        # The path rules below are POSIX, where normcase is the identity;
        # fnmatch measures it against the reference.
        return path

path = _Path()

# From CPython 3b564385e4c9 Lib/os.py; PSF License.
def fspath(path):
    """Return the path representation of a path-like object.

    If str or bytes is passed in, it is returned unchanged. Otherwise the
    os.PathLike interface is used to get the path representation. If the
    path representation is not str or bytes, TypeError is raised. If the
    provided path is not str, bytes, or os.PathLike, TypeError is raised.
    """
    if isinstance(path, (str, bytes)):
        return path

    # Work from the object's type to match method resolution of other magic
    # methods.
    path_type = type(path)
    try:
        path_repr = path_type.__fspath__(path)
    except AttributeError:
        if hasattr(path_type, '__fspath__'):
            raise
        else:
            raise TypeError("expected str, bytes or os.PathLike object, "
                            "not " + path_type.__name__)
    except TypeError:
        if path_type.__fspath__ is None:
            raise TypeError("expected str, bytes or os.PathLike object, "
                            "not " + path_type.__name__) from None
        else:
            raise
    if isinstance(path_repr, (str, bytes)):
        return path_repr
    else:
        raise TypeError("expected {}.__fspath__() to return str or bytes, "
                        "not {}".format(path_type.__name__,
                                        type(path_repr).__name__))


# os.stat models the platform C module; copied library sources use this surface.
class stat_result(tuple):
    def __new__(cls, values):
        return tuple.__new__(cls, tuple(values[:7]) + tuple(int(value) for value in values[7:10]))

    def __init__(self, values):
        self.st_mode, self.st_ino, self.st_dev, self.st_nlink, self.st_uid, self.st_gid, self.st_size, self.st_atime, self.st_mtime, self.st_ctime = values[:10]
        self.st_atime_ns, self.st_mtime_ns, self.st_ctime_ns = values[10:13]

def stat(path, *, dir_fd=None, follow_symlinks=True):
    if dir_fd is not None:
        raise NotImplementedError('stat: dir_fd is not supported')
    path = fspath(path)
    if isinstance(path, bytes):
        path = path.decode('utf-8', 'surrogateescape')
    return stat_result(__file_stat(path, follow_symlinks))

def lstat(path, *, dir_fd=None):
    return stat(path, dir_fd=dir_fd, follow_symlinks=False)
