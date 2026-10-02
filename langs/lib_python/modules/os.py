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
    return __random('pid')

def urandom(size):
    # The system's own disorder, lent by the kernel (ext.builtin._random)
    # as so many bytes. The size is asked for as an index, as the
    # reference asks for it; one outside what a place's count can hold is
    # refused before the kernel is asked, as the reference refuses it.
    if type(size) is not int:
        if getattr(type(size), '__index__', None) is None:
            raise TypeError("'{}' object cannot be interpreted as an integer".format(type(size).__name__))
        from operator import index
        size = index(size)
    if size > 9223372036854775807 or size < -9223372036854775808:
        raise OverflowError('Python int too large to convert to C ssize_t')
    if size < 0:
        raise ValueError('negative argument not allowed')
    return __random('bytes', size)

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

# Additional POSIX APIs use the host's metadata and descriptor operations.
_host_posix = __posix
F_OK = 0
R_OK = 4
W_OK = 2
X_OK = 1
O_RDONLY = 0
O_WRONLY = 1
O_RDWR = 2
O_CREAT = 64
O_EXCL = 128
O_TRUNC = 512
O_APPEND = 1024
O_NONBLOCK = 2048
O_DIRECTORY = 65536
O_NOFOLLOW = 131072
O_CLOEXEC = 524288
supports_dir_fd = set()
supports_fd = set()
supports_follow_symlinks = set()

def fspath(path):
    if isinstance(path, (str, bytes)):
        return path
    method = getattr(type(path), '__fspath__', None)
    if method is None:
        raise TypeError('expected str, bytes or os.PathLike object, not ' + type(path).__name__)
    result = method(path)
    if not isinstance(result, (str, bytes)):
        raise TypeError('expected __fspath__() to return str or bytes, not ' + type(result).__name__)
    return result

def fsdecode(filename):
    filename = fspath(filename)
    return filename.decode('utf-8', 'surrogateescape') if isinstance(filename, bytes) else filename

def fsencode(filename):
    filename = fspath(filename)
    return filename.encode('utf-8', 'surrogateescape') if isinstance(filename, str) else filename

def _posix_call(operation, path, *args):
    result, error = _host_posix(operation, fsdecode(path) if operation != 'close' else path, *args)
    if error is not None:
        number, message = error
        message = message.split(' (os error')[0]
        error_type = {1: PermissionError, 2: FileNotFoundError, 13: PermissionError, 17: FileExistsError, 20: NotADirectoryError, 21: IsADirectoryError}.get(number, OSError)
        raise error_type(number, message, path)
    return result

class stat_result(tuple):
    def __new__(cls, sequence):
        if len(sequence) != 10:
            raise TypeError('os.stat_result() takes a 10-sequence')
        return tuple.__new__(cls, sequence)

    @property
    def st_mode(self): return self[0]
    @property
    def st_ino(self): return self[1]
    @property
    def st_dev(self): return self[2]
    @property
    def st_nlink(self): return self[3]
    @property
    def st_uid(self): return self[4]
    @property
    def st_gid(self): return self[5]
    @property
    def st_size(self): return self[6]
    @property
    def st_atime(self): return self[7] + getattr(self, '_atime_ns', 0) / 1000000000
    @property
    def st_mtime(self): return self[8] + getattr(self, '_mtime_ns', 0) / 1000000000
    @property
    def st_ctime(self): return self[9] + getattr(self, '_ctime_ns', 0) / 1000000000
    @property
    def st_atime_ns(self): return self[7] * 1000000000 + getattr(self, '_atime_ns', 0)
    @property
    def st_mtime_ns(self): return self[8] * 1000000000 + getattr(self, '_mtime_ns', 0)
    @property
    def st_ctime_ns(self): return self[9] * 1000000000 + getattr(self, '_ctime_ns', 0)

def stat(path, *, dir_fd=None, follow_symlinks=True):
    if dir_fd is not None:
        raise NotImplementedError('directory descriptors are not supported')
    parts = _posix_call('stat' if follow_symlinks else 'lstat', path)
    result = stat_result(parts[:10])
    result._atime_ns, result._mtime_ns, result._ctime_ns = parts[10:]
    return result

def lstat(path, *, dir_fd=None):
    return stat(path, dir_fd=dir_fd, follow_symlinks=False)

def rmdir(path, *, dir_fd=None):
    if dir_fd is not None:
        raise NotImplementedError('directory descriptors are not supported')
    return _posix_call('rmdir', path)

def readlink(path, *, dir_fd=None):
    if dir_fd is not None:
        raise NotImplementedError('directory descriptors are not supported')
    value = _posix_call('readlink', path)
    return fsencode(value) if isinstance(fspath(path), bytes) else value

def open(path, flags, mode=511, *, dir_fd=None):
    if dir_fd is not None:
        raise NotImplementedError('directory descriptors are not supported')
    from operator import index
    return _posix_call('open', path, index(flags), index(mode))

def close(fd):
    from operator import index
    return _posix_call('close', index(fd))

def access(path, mode, *, dir_fd=None, effective_ids=False, follow_symlinks=True):
    if dir_fd is not None or effective_ids or not follow_symlinks:
        raise NotImplementedError('extended access options are not supported')
    from operator import index
    return _posix_call('access', path, index(mode))

class DirEntry:
    def __init__(self, directory, name):
        self.name = name
        self.path = path.join(directory, name)
    def __fspath__(self): return self.path
    def stat(self, *, follow_symlinks=True): return stat(self.path, follow_symlinks=follow_symlinks)
    def inode(self): return self.stat(follow_symlinks=False).st_ino
    def is_symlink(self):
        import stat as kinds
        return kinds.S_ISLNK(self.stat(follow_symlinks=False).st_mode)
    def is_dir(self, *, follow_symlinks=True):
        import stat as kinds
        try: return kinds.S_ISDIR(self.stat(follow_symlinks=follow_symlinks).st_mode)
        except FileNotFoundError: return False
    def is_file(self, *, follow_symlinks=True):
        import stat as kinds
        try: return kinds.S_ISREG(self.stat(follow_symlinks=follow_symlinks).st_mode)
        except FileNotFoundError: return False

class _Scandir:
    def __init__(self, directory):
        self.directory = directory
        self.names = iter(listdir(directory))
        self.closed = False
    def __iter__(self): return self
    def __next__(self):
        if self.closed: raise StopIteration
        return DirEntry(self.directory, next(self.names))
    def close(self): self.closed = True
    def __enter__(self): return self
    def __exit__(self, *error): self.close()

def scandir(path='.'):
    return _Scandir(path)

_walk_symlinks_as_files = object()

def walk(top, topdown=True, onerror=None, followlinks=False):
    top = fspath(top)
    try:
        with scandir(top) as entries:
            dirs, files = [], []
            for entry in entries:
                try:
                    is_directory = entry.is_dir()
                    if followlinks is _walk_symlinks_as_files and entry.is_symlink():
                        is_directory = False
                except OSError:
                    is_directory = False
                (dirs if is_directory else files).append(entry.name)
    except OSError as error:
        if onerror is not None: onerror(error)
        return
    if topdown: yield top, dirs, files
    for directory in dirs:
        child = path.join(top, directory)
        if followlinks or not path.islink(child):
            yield from walk(child, topdown, onerror, followlinks)
    if not topdown: yield top, dirs, files

def _path_islink(name):
    import stat as kinds
    try: return kinds.S_ISLNK(lstat(name).st_mode)
    except OSError: return False
path.islink = _path_islink

supports_follow_symlinks.add(stat)
