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

# POSIX open flags for the Linux host interface.
O_RDONLY = 0
O_WRONLY = 1
O_RDWR = 2
O_CREAT = 64
O_EXCL = 128
O_TRUNC = 512
O_APPEND = 1024
O_CLOEXEC = 524288
O_DIRECTORY = 16384
O_NOFOLLOW = 32768
O_NONBLOCK = 2048


def fspath(path):
    if isinstance(path, (str, bytes)):
        return path
    try:
        result = type(path).__fspath__(path)
    except AttributeError:
        raise TypeError('expected str, bytes or os.PathLike object, not ' + type(path).__name__)
    if not isinstance(result, (str, bytes)):
        raise TypeError('__fspath__() must return str or bytes')
    return result

from _os_pathlike import PathLike


class stat_result(tuple):
    n_sequence_fields = 10
    n_fields = 19
    n_unnamed_fields = 3

    def __new__(cls, sequence, dict=None):
        values = tuple(sequence)
        if len(values) < 10 or len(values) > 19:
            raise TypeError('os.stat_result() takes a 10-sequence')
        result = tuple.__new__(cls, values[:10])
        names = ('st_atime', 'st_mtime', 'st_ctime', 'st_atime_ns', 'st_mtime_ns', 'st_ctime_ns', 'st_blksize', 'st_blocks', 'st_rdev')
        for i, name in enumerate(names):
            default = values[7+i] if i < 3 else None
            value = values[10+i] if len(values) > 10+i else (dict or {}).get(name, default)
            setattr(result, name, value)
        return result

    st_mode = property(lambda self: self[0])
    st_ino = property(lambda self: self[1])
    st_dev = property(lambda self: self[2])
    st_nlink = property(lambda self: self[3])
    st_uid = property(lambda self: self[4])
    st_gid = property(lambda self: self[5])
    st_size = property(lambda self: self[6])


def stat(path, *, dir_fd=None, follow_symlinks=True):
    if dir_fd is not None or isinstance(path, int):
        raise NotImplementedError('stat file descriptors are unavailable')
    path = fspath(path)
    fields = _host_file_kind(path, bool(follow_symlinks))
    if len(fields) == 2:
        raise OSError(fields[0], fields[1], path)
    return stat_result(fields)


def lstat(path, *, dir_fd=None):
    return stat(path, dir_fd=dir_fd, follow_symlinks=False)

class terminal_size(tuple):
    def __new__(cls, sequence):
        values = tuple(sequence)
        if len(values) != 2:
            raise TypeError('os.terminal_size() takes a 2-sequence')
        return tuple.__new__(cls, values)
    columns = property(lambda self: self[0])
    lines = property(lambda self: self[1])


def readlink(path, *, dir_fd=None):
    if dir_fd is not None:
        raise NotImplementedError('os.readlink directory descriptors are not supported')
    path = fspath(path)
    raw = _host_file_kind(fsdecode(path), None)
    if isinstance(raw, tuple):
        raise OSError(raw[0], raw[1], path)
    return fsencode(raw) if isinstance(path, bytes) else raw


def get_inheritable(fd):
    if not isinstance(fd, int):
        raise TypeError('an integer is required')
    result = _host_file_kind(fd, 'inheritable')
    if isinstance(result, tuple):
        raise OSError(result[0], result[1])
    return result


def set_inheritable(fd, inheritable):
    if not isinstance(fd, int):
        raise TypeError('an integer is required')
    result = _host_file_kind(fd, 'inheritable', bool(inheritable))
    if isinstance(result, tuple):
        raise OSError(result[0], result[1])
