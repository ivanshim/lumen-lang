# Native POSIX adapter for CPython v3.14.8 Modules/posixmodule.c.
# Copyright (c) Python Software Foundation; PSF License in tests/python/LICENSE.
_host = __posix
import operator as _operator


def _call(op, *args, filename=None, filename2=None):
    error, result = _host(op, *args)
    if error:
        message = strerror(error)
        kind = {2: FileNotFoundError, 17: FileExistsError, 20: NotADirectoryError,
                21: IsADirectoryError, 13: PermissionError, 1: PermissionError,
                4: InterruptedError, 32: BrokenPipeError, 11: BlockingIOError,
                10: ChildProcessError, 3: ProcessLookupError, 110: TimeoutError,
                103: ConnectionAbortedError, 111: ConnectionRefusedError,
                104: ConnectionResetError}.get(error, OSError)
        if filename2 is not None:
            raise kind(error, message, filename, None, filename2)
        if filename is not None:
            raise kind(error, message, filename)
        raise kind(error, message)
    return result


def fspath(path):
    return _host('fspath', path)


def _path(path):
    path = fspath(path)
    if isinstance(path, str):
        path.encode('utf-8', 'surrogateescape')
        if '\0' in path:
            raise ValueError('embedded null character')
    elif b'\0' in path:
        raise ValueError('embedded null byte')
    return path


def _fd(value):
    return _operator.index(value)


def _cint(value):
    number = _operator.index(value)
    if number < -2147483648 or number > 2147483647:
        raise OverflowError('Python int too large to convert to C int')
    return number


def _is_fd_path(value):
    return not isinstance(value, (str, bytes)) and hasattr(type(value), '__index__')


def _descriptor(value):
    if isinstance(value, bool):
        import warnings
        warnings.warn('bool is used as a file descriptor', RuntimeWarning, stacklevel=2)
    number = _operator.index(value)
    if number > 2147483647:
        raise OverflowError('fd is greater than maximum')
    if number < -2147483648:
        raise OverflowError('fd is less than minimum')
    return number


def _dir(value):
    return -100 if value is None else _descriptor(value)


def _decode(value):
    return value.decode('utf-8', 'surrogateescape')


def strerror(code, /):
    return _call('strerror', _cint(code))


(O_RDONLY, O_WRONLY, O_RDWR, O_APPEND, O_CREAT, O_EXCL, O_TRUNC,
 O_NONBLOCK, O_CLOEXEC, O_DIRECTORY, O_NOFOLLOW, SEEK_SET, SEEK_CUR,
 SEEK_END, F_OK, R_OK, W_OK, X_OK) = _call('constants')

# These feature names describe implemented host entry points, not test expectations.
_have_functions = ['HAVE_FACCESSAT', 'HAVE_FCHMODAT', 'HAVE_FSTATAT', 'HAVE_LSTAT',
                   'HAVE_MKDIRAT', 'HAVE_MKFIFOAT', 'HAVE_OPENAT', 'HAVE_READLINKAT',
                   'HAVE_RENAMEAT', 'HAVE_SYMLINKAT', 'HAVE_UNLINKAT', 'HAVE_UTIMENSAT',
                   'HAVE_FCHDIR', 'HAVE_FCHMOD', 'HAVE_FTRUNCATE', 'HAVE_LINKAT',
                   'HAVE_FDOPENDIR', 'HAVE_FUTIMENS']
environ = _call('environ')


_stat_dict_missing = object()


class stat_result(tuple):
    __slots__ = ()
    n_sequence_fields = 10
    n_fields = 19
    n_unnamed_fields = 3

    def __new__(cls, sequence, dict=_stat_dict_missing):
        if dict is not _stat_dict_missing and not isinstance(dict, type({})):
            raise TypeError('os.stat_result() takes a dict as second arg, if any')
        values = tuple(sequence)
        if len(values) < 10:
            raise TypeError('os.stat_result() takes an at least 10-sequence (' + str(len(values)) + '-sequence given)')
        if len(values) > 19:
            raise TypeError('os.stat_result() takes an at most 19-sequence (' + str(len(values)) + '-sequence given)')
        extra = {} if dict is _stat_dict_missing else dict
        times = tuple(values[i] if len(values) > i else extra.get(name, values[i - 3]) for i, name in [(10, 'st_atime'), (11, 'st_mtime'), (12, 'st_ctime')])
        nanos = tuple(values[i] if len(values) > i else extra.get(name) for i, name in [(13, 'st_atime_ns'), (14, 'st_mtime_ns'), (15, 'st_ctime_ns')])
        host_fields = tuple(values[i] if len(values) > i else extra.get(name) for i, name in [(16, 'st_blksize'), (17, 'st_blocks'), (18, 'st_rdev')])
        return _host('structseq_new', cls, values[:10], times + nanos + host_fields)

    st_mode = property(lambda self: self[0])
    st_ino = property(lambda self: self[1])
    st_dev = property(lambda self: self[2])
    st_nlink = property(lambda self: self[3])
    st_uid = property(lambda self: self[4])
    st_gid = property(lambda self: self[5])
    st_size = property(lambda self: self[6])
    st_atime = property(lambda self: _host('structseq_get', self, 0))
    st_mtime = property(lambda self: _host('structseq_get', self, 1))
    st_ctime = property(lambda self: _host('structseq_get', self, 2))
    st_atime_ns = property(lambda self: _host('structseq_get', self, 3))
    st_mtime_ns = property(lambda self: _host('structseq_get', self, 4))
    st_ctime_ns = property(lambda self: _host('structseq_get', self, 5))
    st_blksize = property(lambda self: _host('structseq_get', self, 6))
    st_blocks = property(lambda self: _host('structseq_get', self, 7))
    st_rdev = property(lambda self: _host('structseq_get', self, 8))

    def __repr__(self):
        names = ('st_mode', 'st_ino', 'st_dev', 'st_nlink', 'st_uid', 'st_gid', 'st_size', 'st_atime', 'st_mtime', 'st_ctime')
        return 'os.stat_result(' + ', '.join(name + '=' + repr(value) for name, value in zip(names, self)) + ')'

    def __reduce__(self):
        names = ('st_atime', 'st_mtime', 'st_ctime', 'st_atime_ns', 'st_mtime_ns', 'st_ctime_ns', 'st_blksize', 'st_blocks', 'st_rdev')
        return (type(self), (tuple(self), {name: getattr(self, name) for name in names}))

    def __init_subclass__(cls, **kwargs):
        raise TypeError("type 'os.stat_result' is not an acceptable base type")

stat_result.__module__ = 'os'


def _stat(data):
    nanos = tuple(data[i] * 1000000000 + data[i + 3] for i in (7, 8, 9))
    times = tuple(data[i] + data[i + 3] / 1000000000 for i in (7, 8, 9))
    return stat_result(data[:10] + times + nanos + data[13:16])


def stat(path, *, dir_fd=None, follow_symlinks=True):
    if _is_fd_path(path):
        if dir_fd is not None:
            raise ValueError("stat: can't specify dir_fd without matching path")
        if not follow_symlinks:
            raise ValueError('stat: cannot use fd and follow_symlinks together')
        return fstat(_descriptor(path))
    original = _path(path)
    return _stat(_call('stat', original, _dir(dir_fd), int(bool(follow_symlinks)), filename=original))


def lstat(path, *, dir_fd=None):
    original = _path(path)
    return _stat(_call('lstat', original, _dir(dir_fd), 0, filename=original))


def fstat(fd):
    return _stat(_call('fstat', _cint(fd)))


def getcwdb():
    return _call('cwd')


def getcwd():
    return _decode(getcwdb())


def chdir(path):
    if _is_fd_path(path):
        _call('fchdir', _descriptor(path))
    else:
        original = _path(path)
        _call('chdir', original, filename=original)


def fchdir(fd):
    _call('fchdir', _descriptor(fd))


def listdir(path=None):
    if path is None:
        path = '.'
    original = _descriptor(path) if _is_fd_path(path) else _path(path)
    result = []
    with scandir(original) as entries:
        for entry in entries:
            result.append(entry.name)
    return result


def mkdir(path, mode=0o777, *, dir_fd=None):
    original = _path(path)
    _call('mkdir', original, _cint(mode), _dir(dir_fd), filename=original)


def rmdir(path, *, dir_fd=None):
    original = _path(path)
    _call('rmdir', original, _dir(dir_fd), filename=original)


def unlink(path, *, dir_fd=None):
    original = _path(path)
    _call('unlink', original, _dir(dir_fd), filename=original)


def remove(path, *, dir_fd=None):
    return unlink(path, dir_fd=dir_fd)


def rename(src, dst, *, src_dir_fd=None, dst_dir_fd=None):
    src, dst = _path(src), _path(dst)
    _call('rename', src, dst, _dir(src_dir_fd), _dir(dst_dir_fd), filename=src, filename2=dst)


def replace(src, dst, *, src_dir_fd=None, dst_dir_fd=None):
    return rename(src, dst, src_dir_fd=src_dir_fd, dst_dir_fd=dst_dir_fd)


def open(path, flags, mode=0o777, *, dir_fd=None):
    original = _path(path)
    return _call('open', original, _cint(flags), _cint(mode), _dir(dir_fd), filename=original)


def close(fd):
    _call('close', _cint(fd))


def dup(fd):
    return _call('dup', _cint(fd))


def read(fd, length, /):
    return _call('read', _cint(fd), _fd(length))


def write(fd, data, /):
    descriptor = _cint(fd)
    if not isinstance(data, (bytes, bytearray)):
        try:
            view = memoryview(data)
        except TypeError:
            raise TypeError("a bytes-like object is required, not '" + type(data).__name__ + "'")
        if hasattr(view, 'c_contiguous') and not view.c_contiguous:
            raise BufferError('memoryview: underlying buffer is not C-contiguous')
        data = bytes(view)
    return _call('write', descriptor, bytes(data))


def lseek(fd, position, whence, /):
    return _call('lseek', _cint(fd), _fd(position), _cint(whence))


def ftruncate(fd, length, /):
    _call('ftruncate', _cint(fd), _fd(length))


def readlink(path, *, dir_fd=None):
    original = _path(path)
    result = _call('readlink', original, _dir(dir_fd), filename=original)
    return result if isinstance(original, bytes) else _decode(result)


def symlink(src, dst, target_is_directory=False, *, dir_fd=None):
    src, dst = _path(src), _path(dst)
    _call('symlink', src, dst, _dir(dir_fd), filename=src, filename2=dst)


def link(src, dst, *, src_dir_fd=None, dst_dir_fd=None, follow_symlinks=True):
    src, dst = _path(src), _path(dst)
    _call('link', src, dst, _dir(src_dir_fd), _dir(dst_dir_fd), int(bool(follow_symlinks)), filename=src, filename2=dst)


def access(path, mode, *, dir_fd=None, effective_ids=False, follow_symlinks=True):
    original = _path(path)
    return _call('access', original, _cint(mode), _dir(dir_fd), int(bool(effective_ids)), int(bool(follow_symlinks)))


def chmod(path, mode, *, dir_fd=None, follow_symlinks=True):
    if _is_fd_path(path):
        if dir_fd is not None or not follow_symlinks:
            raise ValueError('chmod: cannot use fd and follow_symlinks together')
        return fchmod(path, mode)
    original = _path(path)
    try:
        _call('chmod', original, _cint(mode), _dir(dir_fd), int(bool(follow_symlinks)), filename=original)
    except OSError as error:
        if not follow_symlinks and error.errno == 95:
            if dir_fd is not None:
                raise ValueError('chmod: cannot use dir_fd and follow_symlinks together')
            raise NotImplementedError('chmod: follow_symlinks unavailable on this platform')
        raise


def fchmod(fd, mode):
    _call('fchmod', _cint(fd), _cint(mode))


def umask(mask, /):
    return _call('umask', _cint(mask))


def utime(path, times=None, *, ns=None, dir_fd=None, follow_symlinks=True):
    if times is not None and ns is not None:
        raise ValueError("utime: you may specify either 'times' or 'ns' but not both")
    if ns is not None:
        if not isinstance(ns, tuple) or len(ns) != 2:
            raise TypeError('utime: \'ns\' must be a tuple of two ints')
        moments = [_fd(n) for n in ns]
    elif times is not None:
        if not isinstance(times, tuple) or len(times) != 2:
            raise TypeError('utime: \'times\' must be either a tuple of two ints or None')
        moments = [int(n * 1000000000) for n in times]
    else:
        moments = [0, 0]
    sec1, nano1 = divmod(moments[0], 1000000000)
    sec2, nano2 = divmod(moments[1], 1000000000)
    if _is_fd_path(path):
        if dir_fd is not None or not follow_symlinks:
            raise ValueError('utime: cannot use fd and follow_symlinks together')
        _call('futime', _descriptor(path), sec1, nano1, sec2, nano2, int(times is not None or ns is not None))
        return
    original = _path(path)
    _call('utime', original, sec1, nano1, sec2, nano2, _dir(dir_fd), int(times is not None or ns is not None), int(bool(follow_symlinks)), filename=original)


def putenv(name, value, /):
    name, value = _path(name), _path(value)
    if (b'=' if isinstance(name, bytes) else '=') in name:
        raise ValueError('illegal environment variable name')
    _call('putenv', name, value)


def unsetenv(name, /):
    _call('unsetenv', name)


def getpid():
    return _call('getpid')


def getuid():
    return _call('getuid')


def geteuid():
    return _call('geteuid')


def getgid():
    return _call('getgid')


def getegid():
    return _call('getegid')


def urandom(size, /):
    size = _fd(size)
    if size < 0:
        raise ValueError('negative argument not allowed')
    return _call('urandom', size)


def cpu_count():
    return _call('cpu_count')


def isatty(fd, /):
    return _call('isatty', _cint(fd))


def device_encoding(fd):
    return 'UTF-8' if isatty(fd) else None


def mkfifo(path, mode=0o666, *, dir_fd=None):
    original = _path(path)
    _call('mkfifo', original, _cint(mode), _dir(dir_fd), filename=original)


class uname_result(tuple):
    def __new__(cls, sequence):
        if len(sequence) != 5:
            raise TypeError('os.uname_result() takes a 5-sequence')
        return tuple.__new__(cls, sequence)
    sysname = property(lambda self: self[0])
    nodename = property(lambda self: self[1])
    release = property(lambda self: self[2])
    version = property(lambda self: self[3])
    machine = property(lambda self: self[4])


def uname():
    return uname_result(_call('uname'))


class terminal_size(tuple):
    def __new__(cls, sequence):
        if len(sequence) != 2:
            raise TypeError('os.terminal_size() takes a 2-sequence')
        return tuple.__new__(cls, sequence)
    columns = property(lambda self: self[0])
    lines = property(lambda self: self[1])


def get_terminal_size(fd=1):
    return terminal_size(_call('terminal', _cint(fd)))


def waitstatus_to_exitcode(status):
    status = _fd(status)
    if status < -2147483648 or status > 2147483647:
        raise OverflowError('Python int too large to convert to C int')
    kind, result = _call('waitstatus', status)
    if kind == 0:
        return result
    if kind == 1:
        raise ValueError('process stopped by delivery of signal ' + str(result))
    raise ValueError('invalid wait status: ' + str(result))


class DirEntry:
    __slots__ = ('_name', '_path', '_inode', '_type', '_dir_fd', '_stat', '_lstat')
    def __init__(self, directory, name, inode, kind):
        import posixpath
        self._name = name
        self._dir_fd = directory if isinstance(directory, int) else None
        self._path = name if self._dir_fd is not None else posixpath.join(directory, name)
        self._inode = inode
        self._type = kind
        self._stat = None
        self._lstat = None
    name = property(lambda self: self._name)
    path = property(lambda self: self._path)
    def __fspath__(self):
        return self.path
    def inode(self):
        return self._inode
    def stat(self, *, follow_symlinks=True):
        if follow_symlinks:
            if self._stat is None:
                if self._type != 10 and self._lstat is not None:
                    self._stat = self._lstat
                else:
                    self._stat = stat(self.path, dir_fd=self._dir_fd)
            return self._stat
        if self._lstat is None:
            if self._type != 10 and self._stat is not None:
                self._lstat = self._stat
            else:
                self._lstat = lstat(self.path, dir_fd=self._dir_fd)
        return self._lstat
    def is_symlink(self):
        if self._type:
            return self._type == 10
        import stat
        try:
            return stat.S_ISLNK(self.stat(follow_symlinks=False).st_mode)
        except FileNotFoundError:
            return False
    def is_file(self, *, follow_symlinks=True):
        if self._type and (self._type != 10 or not follow_symlinks):
            return self._type == 8
        import stat
        try:
            return stat.S_ISREG(self.stat(follow_symlinks=follow_symlinks).st_mode)
        except FileNotFoundError:
            return False
    def is_dir(self, *, follow_symlinks=True):
        if self._type and (self._type != 10 or not follow_symlinks):
            return self._type == 4
        import stat
        try:
            return stat.S_ISDIR(self.stat(follow_symlinks=follow_symlinks).st_mode)
        except FileNotFoundError:
            return False
    def __repr__(self):
        return '<DirEntry ' + repr(self.name) + '>'

class _ScandirIterator:
    def __init__(self, path):
        self._closed = True
        self._path = path
        self._handle = _call('scandir', path, filename=None if _is_fd_path(path) else path)
        self._closed = False
    def __iter__(self):
        return self
    def __next__(self):
        if self._closed:
            raise StopIteration
        try:
            record = _call('scandir_next', self._handle)
        except OSError:
            self.close()
            raise
        if record is None:
            self.close()
            raise StopIteration
        name, inode, kind = record
        if not isinstance(self._path, bytes):
            name = _decode(name)
        return DirEntry(self._path, name, inode, kind)
    def close(self):
        if not self._closed:
            self._closed = True
            _call('scandir_close', self._handle)
    def __enter__(self):
        return self
    def __exit__(self, *args):
        self.close()
    def __del__(self):
        self.close()

def scandir(path=None):
    if path is None:
        path = '.'
    return _ScandirIterator(_descriptor(path) if _is_fd_path(path) else _path(path))
