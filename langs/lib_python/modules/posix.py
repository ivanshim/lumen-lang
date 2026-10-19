# Native POSIX adapter for CPython v3.14.8 Modules/posixmodule.c.
# Copyright (c) Python Software Foundation; PSF License in tests/python/LICENSE.
_host = __posix
import operator as _operator


_ERROR_KINDS = {2: FileNotFoundError, 17: FileExistsError, 20: NotADirectoryError,
                21: IsADirectoryError, 13: PermissionError, 1: PermissionError,
                4: InterruptedError, 32: BrokenPipeError, 11: BlockingIOError,
                10: ChildProcessError, 3: ProcessLookupError, 110: TimeoutError,
                103: ConnectionAbortedError, 111: ConnectionRefusedError,
                104: ConnectionResetError}


def _raise(error, filename=None, filename2=None):
    message = strerror(error)
    kind = _ERROR_KINDS.get(error, OSError)
    if filename2 is not None:
        raise kind(error, message, filename, None, filename2)
    if filename is not None:
        raise kind(error, message, filename)
    raise kind(error, message)


def _call(op, *args, filename=None, filename2=None):
    error, result = _host(op, *args)
    if error:
        _raise(error, filename, filename2)
    return result


def fspath(path):
    if type(path) is str or type(path) is bytes:
        return path
    return _host('fspath', path)


def _path(path):
    if type(path) is str:
        path.encode('utf-8', 'surrogateescape')
        if '\0' in path:
            raise ValueError('embedded null character')
        return path
    path = fspath(path)
    if isinstance(path, str):
        path.encode('utf-8', 'surrogateescape')
        if '\0' in path:
            raise ValueError('embedded null character')
    elif b'\0' in path:
        raise ValueError('embedded null byte')
    return path


def _fd(value):
    if type(value) is int:
        return value
    return _operator.index(value)


def _cint(value):
    number = value if type(value) is int else _operator.index(value)
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


# The configuration strings the host library answers for. The list
# follows the names glibc gives them; an integer is passed through.
_confstr_names = {
    'CS_PATH': 0,
    'CS_V6_WIDTH_RESTRICTED_ENVS': 1,
    'CS_GNU_LIBC_VERSION': 2,
    'CS_GNU_LIBPTHREAD_VERSION': 3,
    'CS_V5_WIDTH_RESTRICTED_ENVS': 4,
    'CS_V7_WIDTH_RESTRICTED_ENVS': 5,
}


def confstr(name, /):
    if isinstance(name, str):
        try:
            name = _confstr_names[name]
        except KeyError:
            raise ValueError('unrecognized configuration name') from None
    return _call('confstr', _cint(name))


(O_RDONLY, O_WRONLY, O_RDWR, O_APPEND, O_CREAT, O_EXCL, O_TRUNC,
 O_NONBLOCK, O_CLOEXEC, O_DIRECTORY, O_NOFOLLOW, SEEK_SET, SEEK_CUR,
 SEEK_END, F_OK, R_OK, W_OK, X_OK, O_TMPFILE, O_NOCTTY) = _call('constants')

# These feature names describe implemented host entry points, not test expectations.
_have_functions = ['HAVE_FACCESSAT', 'HAVE_FCHMODAT', 'HAVE_FSTATAT', 'HAVE_LSTAT',
                   'HAVE_MKDIRAT', 'HAVE_MKFIFOAT', 'HAVE_OPENAT', 'HAVE_READLINKAT',
                   'HAVE_RENAMEAT', 'HAVE_SYMLINKAT', 'HAVE_UNLINKAT', 'HAVE_UTIMENSAT',
                   'HAVE_FCHDIR', 'HAVE_FCHMOD', 'HAVE_FTRUNCATE', 'HAVE_LINKAT',
                   'HAVE_FDOPENDIR', 'HAVE_FUTIMENS', 'HAVE_FCHOWNAT', 'HAVE_FCHOWN',
                   'HAVE_LCHOWN', 'HAVE_FSTATVFS']
environ = _call('environ')


_stat_dict_missing = object()


_stat_extra_fields = ('st_atime', 'st_mtime', 'st_ctime', 'st_atime_ns',
                      'st_mtime_ns', 'st_ctime_ns', 'st_blksize', 'st_blocks',
                      'st_rdev')


class stat_result(tuple):
    __slots__ = ()
    n_sequence_fields = 10
    n_fields = 19
    n_unnamed_fields = 3
    __match_args__ = ('st_mode', 'st_ino', 'st_dev', 'st_nlink', 'st_uid',
                      'st_gid', 'st_size')

    def __new__(cls, sequence, dict=_stat_dict_missing):
        if dict is not _stat_dict_missing and not isinstance(dict, type({})):
            raise TypeError('os.stat_result() takes a dict as second arg, if any')
        values = tuple(sequence)
        if len(values) < 10:
            raise TypeError('os.stat_result() takes an at least 10-sequence (' + str(len(values)) + '-sequence given)')
        if len(values) > 19:
            raise TypeError('os.stat_result() takes an at most 19-sequence (' + str(len(values)) + '-sequence given)')
        changes = {} if dict is _stat_dict_missing else dict
        row = list(values[:19])
        found = 0
        for at in range(10, 19):
            if at < len(row):
                continue
            name = _stat_extra_fields[at - 10]
            if name in changes:
                row.append(changes[name])
                found += 1
            elif at - 3 < 10:
                row.append(row[at - 3])
            else:
                row.append(None)
        if len(changes) > found:
            raise TypeError('os.stat_result() got duplicate or unexpected field name(s)')
        return _host('structseq_new', cls, values[:10], tuple(row[10:19]))

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

    def __replace__(self, **changes):
        raise TypeError('__replace__() is not supported for os.stat_result because it has unnamed field(s)')

    def __init_subclass__(cls, **kwargs):
        raise TypeError("type 'os.stat_result' is not an acceptable base type")

stat_result.__module__ = 'os'


class times_result(tuple):
    __slots__ = ()
    n_sequence_fields = 5
    n_fields = 5
    n_unnamed_fields = 0
    __match_args__ = ('user', 'system', 'children_user', 'children_system', 'elapsed')

    def __new__(cls, sequence):
        values = tuple(sequence)
        if len(values) != 5:
            raise TypeError('posix.times_result() takes a 5-sequence (' + str(len(values)) + '-sequence given)')
        return tuple.__new__(cls, values)

    user = property(lambda self: self[0])
    system = property(lambda self: self[1])
    children_user = property(lambda self: self[2])
    children_system = property(lambda self: self[3])
    elapsed = property(lambda self: self[4])

    def __repr__(self):
        names = ('user', 'system', 'children_user', 'children_system', 'elapsed')
        return 'posix.times_result(' + ', '.join(name + '=' + repr(value) for name, value in zip(names, self)) + ')'

    def __reduce__(self):
        return (type(self), (tuple(self),))

    def __replace__(self, **changes):
        names = ('user', 'system', 'children_user', 'children_system', 'elapsed')
        values = {}
        for name in names:
            values[name] = changes.pop(name) if name in changes else getattr(self, name)
        if changes:
            raise TypeError('Got unexpected field name(s): ' + repr(list(changes)))
        return type(self)(tuple(values[name] for name in names))


def times():
    return times_result(_call('times'))


def _stat(data):
    # The host hands over all nineteen fields in their final shape, so
    # the sequence checks of stat_result.__new__ have nothing left to decide.
    return _host('structseq_new', stat_result, data[:10], data[10:])


def stat(path, *, dir_fd=None, follow_symlinks=True):
    if type(path) is str:
        # The checks _path makes, without the call.
        path.encode('utf-8', 'surrogateescape')
        if '\0' in path:
            raise ValueError('embedded null character')
        original = path
        if dir_fd is None and type(follow_symlinks) is bool:
            return _host('stat_native', stat_result, path, follow_symlinks)
    elif type(path) is bytes and dir_fd is None and type(follow_symlinks) is bool:
        return _host('stat_native', stat_result, _path(path), follow_symlinks)
    elif _is_fd_path(path):
        if dir_fd is not None:
            raise ValueError("stat: can't specify dir_fd without matching path")
        if not follow_symlinks:
            raise ValueError('stat: cannot use fd and follow_symlinks together')
        return fstat(_descriptor(path))
    else:
        original = _path(path)
    error, data = _host('stat', original, -100 if dir_fd is None else dir_fd if type(dir_fd) is int and -2147483648 <= dir_fd <= 2147483647 else _descriptor(dir_fd), int(bool(follow_symlinks)))
    if error:
        _raise(error, original)
    return _host('structseq_new', stat_result, data[:10], data[10:])


def lstat(path, *, dir_fd=None):
    if type(path) is str:
        path.encode('utf-8', 'surrogateescape')
        if '\0' in path:
            raise ValueError('embedded null character')
        original = path
    else:
        original = _path(path)
    error, data = _host('lstat', original, -100 if dir_fd is None else dir_fd if type(dir_fd) is int and -2147483648 <= dir_fd <= 2147483647 else _descriptor(dir_fd), 0)
    if error:
        _raise(error, original)
    return _host('structseq_new', stat_result, data[:10], data[10:])


def fstat(fd):
    error, data = _host('fstat', fd if type(fd) is int and -2147483648 <= fd <= 2147483647 else _cint(fd))
    if error:
        _raise(error)
    return _host('structseq_new', stat_result, data[:10], data[10:])


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
    # The names straight off the directory stream: the same stream
    # scandir reads, without an entry object made for each name.
    if _is_fd_path(path):
        original = _descriptor(path)
        error, handle = _host('scandir', original)
        if error:
            _raise(error)
    else:
        original = _path(path)
        error, handle = _host('scandir', original)
        if error:
            _raise(error, original)
    text = not isinstance(original, bytes)
    result = []
    try:
        while True:
            error, record = _host('scandir_next', handle)
            if error:
                _raise(error)
            if record is None:
                break
            name = record[0]
            result.append(name.decode('utf-8', 'surrogateescape') if text else name)
    finally:
        _host('scandir_close', handle)
    return result


def mkdir(path, mode=0o777, *, dir_fd=None):
    original = _path(path)
    error, _ = _host('mkdir', original, mode if type(mode) is int and -2147483648 <= mode <= 2147483647 else _cint(mode), -100 if dir_fd is None else dir_fd if type(dir_fd) is int and -2147483648 <= dir_fd <= 2147483647 else _descriptor(dir_fd))
    if error:
        _raise(error, original)


def rmdir(path, *, dir_fd=None):
    original = _path(path)
    error, _ = _host('rmdir', original, -100 if dir_fd is None else dir_fd if type(dir_fd) is int and -2147483648 <= dir_fd <= 2147483647 else _descriptor(dir_fd))
    if error:
        _raise(error, original)


def unlink(path, *, dir_fd=None):
    original = _path(path)
    error, _ = _host('unlink', original, -100 if dir_fd is None else dir_fd if type(dir_fd) is int and -2147483648 <= dir_fd <= 2147483647 else _descriptor(dir_fd))
    if error:
        _raise(error, original)


def remove(path, *, dir_fd=None):
    return unlink(path, dir_fd=dir_fd)


def rename(src, dst, *, src_dir_fd=None, dst_dir_fd=None):
    src, dst = _path(src), _path(dst)
    _call('rename', src, dst, _dir(src_dir_fd), _dir(dst_dir_fd), filename=src, filename2=dst)


def replace(src, dst, *, src_dir_fd=None, dst_dir_fd=None):
    return rename(src, dst, src_dir_fd=src_dir_fd, dst_dir_fd=dst_dir_fd)


def open(path, flags, mode=0o777, *, dir_fd=None):
    original = _path(path)
    error, fd = _host('open', original, flags if type(flags) is int and -2147483648 <= flags <= 2147483647 else _cint(flags), mode if type(mode) is int and -2147483648 <= mode <= 2147483647 else _cint(mode), -100 if dir_fd is None else dir_fd if type(dir_fd) is int and -2147483648 <= dir_fd <= 2147483647 else _descriptor(dir_fd))
    if error:
        _raise(error, original)
    return fd


def close(fd):
    error, _ = _host('close', fd if type(fd) is int and -2147483648 <= fd <= 2147483647 else _cint(fd))
    if error:
        _raise(error)


def dup(fd):
    return _call('dup', _cint(fd))


def read(fd, length, /):
    error, data = _host('read', fd if type(fd) is int and -2147483648 <= fd <= 2147483647 else _cint(fd), length if type(length) is int else _fd(length))
    if error:
        _raise(error)
    return data


def readinto(fd, buffer, /):
    descriptor = _cint(fd)
    view = memoryview(buffer)
    if view.readonly:
        raise TypeError('readinto() argument must be read-write bytes-like object, not ' + type(buffer).__name__)
    if not view.c_contiguous:
        raise BufferError('memoryview: underlying buffer is not C-contiguous')
    octets = view.cast('B')
    data = _call('read', descriptor, len(octets))
    octets[:len(data)] = data
    return len(data)


def write(fd, data, /):
    descriptor = fd if type(fd) is int and -2147483648 <= fd <= 2147483647 else _cint(fd)
    if type(data) is not bytes:
        if not isinstance(data, (bytes, bytearray)):
            try:
                view = memoryview(data)
            except TypeError:
                raise TypeError("a bytes-like object is required, not '" + type(data).__name__ + "'")
            if hasattr(view, 'c_contiguous') and not view.c_contiguous:
                raise BufferError('memoryview: underlying buffer is not C-contiguous')
            data = bytes(view)
        data = bytes(data)
    error, count = _host('write', descriptor, data)
    if error:
        _raise(error)
    return count


def lseek(fd, position, whence, /):
    error, result = _host('lseek', fd if type(fd) is int and -2147483648 <= fd <= 2147483647 else _cint(fd), position if type(position) is int else _fd(position), whence if type(whence) is int and -2147483648 <= whence <= 2147483647 else _cint(whence))
    if error:
        _raise(error)
    return result


def ftruncate(fd, length, /):
    _call('ftruncate', _cint(fd), _fd(length))


def sendfile(out_fd, in_fd, offset, count):
    out, source = _cint(out_fd), _cint(in_fd)
    if offset is None:
        return _call('sendfile', out, source, 0, 0, _fd(count))
    return _call('sendfile', out, source, 1, _fd(offset), _fd(count))


def copy_file_range(src, dst, count, offset_src=None, offset_dst=None):
    return _call('copy_file_range', _cint(src), _cint(dst), _fd(count),
                 int(offset_src is not None), 0 if offset_src is None else _fd(offset_src),
                 int(offset_dst is not None), 0 if offset_dst is None else _fd(offset_dst))


XATTR_CREATE = 1
XATTR_REPLACE = 2
XATTR_SIZE_MAX = 65536


def _xattr(verb, name, path, attribute, value, flags, follow_symlinks):
    if _is_fd_path(path):
        if not follow_symlinks:
            raise ValueError(name + ': cannot use fd and follow_symlinks together')
        target, kind = _cint(path), 2
    else:
        target, kind = _path(path), 0 if follow_symlinks else 1
    label = b'' if attribute is None else _path(attribute)
    return _call('xattr', verb, target, label, value, kind, flags, filename=None if kind == 2 else target)


def getxattr(path, attribute, *, follow_symlinks=True):
    return _xattr(0, 'getxattr', path, attribute, b'', 0, follow_symlinks)


def setxattr(path, attribute, value, flags=0, *, follow_symlinks=True):
    _xattr(1, 'setxattr', path, attribute, bytes(value), _cint(flags), follow_symlinks)


def listxattr(path=None, *, follow_symlinks=True):
    names = _xattr(2, 'listxattr', '.' if path is None else path, None, b'', 0, follow_symlinks)
    return [_decode(name) for name in names.split(b'\0') if name]


def removexattr(path, attribute, *, follow_symlinks=True):
    _xattr(3, 'removexattr', path, attribute, b'', 0, follow_symlinks)


def readlink(path, *, dir_fd=None):
    original = _path(path)
    error, result = _host('readlink', original, -100 if dir_fd is None else dir_fd if type(dir_fd) is int and -2147483648 <= dir_fd <= 2147483647 else _descriptor(dir_fd))
    if error:
        _raise(error, original)
    return result if isinstance(original, bytes) else _decode(result)


def symlink(src, dst, target_is_directory=False, *, dir_fd=None):
    src, dst = _path(src), _path(dst)
    error, _ = _host('symlink', src, dst, -100 if dir_fd is None else dir_fd if type(dir_fd) is int and -2147483648 <= dir_fd <= 2147483647 else _descriptor(dir_fd))
    if error:
        _raise(error, src, dst)


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
    _forget_environment()


def unsetenv(name, /):
    _call('unsetenv', name)
    _forget_environment()


def _forget_environment():
    # sys.flags answers from the environment as last read.
    import sys
    held = getattr(sys, '_environment_values', None)
    if held is not None:
        held.clear()


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
    __slots__ = ()
    n_sequence_fields = 2
    n_fields = 2
    n_unnamed_fields = 0

    # Construct the native terminal record with its fixed sequence size.
    def __new__(cls, sequence, dict=_stat_dict_missing):
        if dict is not _stat_dict_missing and not isinstance(dict, type({})):
            raise TypeError('os.terminal_size() takes a dict as second arg, if any')
        values = tuple(sequence)
        if len(values) != 2:
            raise TypeError('os.terminal_size() takes a 2-sequence (' + str(len(values)) + '-sequence given)')
        return tuple.__new__(cls, values)
    columns = property(lambda self: self[0])
    lines = property(lambda self: self[1])

    # Render the public dimensions as CPython's native record does.
    def __repr__(self):
        return 'os.terminal_size(columns=' + repr(self.columns) + ', lines=' + repr(self.lines) + ')'

    # Refuse subclassing of the native result type.
    def __init_subclass__(cls, **kwargs):
        raise TypeError("type 'os.terminal_size' is not an acceptable base type")

terminal_size.__module__ = 'os'


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
    # Report the absence of Windows junctions on the POSIX filesystem.
    def is_junction(self):
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

_new_entry = object.__new__

# What the host copies to make an entry, giving each of its places a new
# value; the places are set here in the order it fills them.
_entry_sample = _new_entry(DirEntry)
_entry_sample._name = _entry_sample._path = ''
_entry_sample._inode = _entry_sample._type = 0
_entry_sample._dir_fd = _entry_sample._stat = _entry_sample._lstat = None

class _ScandirIterator:
    def __init__(self, path):
        self._closed = True
        self._path = path
        error, handle = _host('scandir', path)
        if type(path) is int:
            if error:
                _raise(error)
            self._dir_fd = path
            self._prefix = None
            self._text = True
        else:
            if error:
                _raise(error, path)
            self._dir_fd = None
            # What posixpath.join puts before each name: the directory,
            # with a separator unless it already ends in one.
            if isinstance(path, bytes):
                self._text = False
                self._prefix = path if path.endswith(b'/') else path + b'/'
            else:
                self._text = True
                self._prefix = path if path.endswith('/') else path + '/'
        # The host makes the entry itself where the names are text.
        self._kind = _entry_sample if self._text else None
        self._handle = handle
        self._closed = False
    def __iter__(self):
        return self
    def __next__(self):
        if self._closed:
            raise StopIteration
        error, entry = _host('scandir_entry', self._handle, self._kind, self._prefix, self._dir_fd)
        if error:
            self.close()
            _raise(error)
        if entry is None:
            self._closed = True
            error = _host('scandir_close', self._handle)[0]
            if error:
                _raise(error)
            raise StopIteration
        if type(entry) is not tuple:
            return entry
        # A name to be decoded here: a bytes path, or text that is not UTF-8.
        name, inode, kind = entry
        if self._text:
            name = name.decode('utf-8', 'surrogateescape')
        entry = _new_entry(DirEntry)
        entry._name = name
        entry._path = name if self._prefix is None else self._prefix + name
        entry._inode = inode
        entry._type = kind
        entry._dir_fd = self._dir_fd
        entry._stat = None
        entry._lstat = None
        return entry
    def close(self):
        if not self._closed:
            self._closed = True
            _call('scandir_close', self._handle)
    def __enter__(self):
        return self
    def __exit__(self, *args):
        if not self._closed:
            self.close()
    def __del__(self):
        if not self._closed:
            self.close()

def scandir(path=None):
    if path is None:
        path = '.'
    if type(path) is str:
        path.encode('utf-8', 'surrogateescape')
        if '\0' in path:
            raise ValueError('embedded null character')
        return _ScandirIterator(path)
    return _ScandirIterator(_descriptor(path) if _is_fd_path(path) else _path(path))


def get_inheritable(fd, /):
    error, result = _host('get_inheritable', fd if type(fd) is int and -2147483648 <= fd <= 2147483647 else _cint(fd))
    if error:
        _raise(error)
    return result


def set_inheritable(fd, inheritable, /):
    error, _ = _host('set_inheritable', fd if type(fd) is int and -2147483648 <= fd <= 2147483647 else _cint(fd), bool(_cint(inheritable)))
    if error:
        _raise(error)


def get_blocking(fd, /):
    return _call('get_blocking', _cint(fd))


def set_blocking(fd, blocking, /):
    _call('set_blocking', _cint(fd), bool(_cint(blocking)))


def pipe():
    return _call('pipe')


# Convert the Linux uid_t/gid_t argument using CPython's signed-long rules.
def _owner_id(value, kind):
    try:
        number = _operator.index(value)
    except BaseException:
        raise TypeError(kind + ' should be integer, not ' + type(value).__name__) from None
    if number < -1 or 4294967295 < number <= 9223372036854775807:
        raise OverflowError(kind + ' is less than minimum')
    if number > 4294967295:
        raise OverflowError(kind + ' is greater than maximum')
    return number


# Change ownership relative to a directory, optionally preserving the final link.
def chown(path, uid, gid, *, dir_fd=None, follow_symlinks=True):
    if _is_fd_path(path):
        if dir_fd is not None:
            raise ValueError("chown: can't specify both dir_fd and fd")
        if not follow_symlinks:
            raise ValueError('chown: cannot use fd and follow_symlinks together')
        return fchown(path, uid, gid)
    original = _path(path)
    _call('chown', original, _owner_id(uid, 'uid'), _owner_id(gid, 'gid'),
          _dir(dir_fd), int(bool(follow_symlinks)), filename=original)


# Change the ownership of an open descriptor.
def fchown(fd, uid, gid):
    _call('fchown', _cint(fd), _owner_id(uid, 'uid'), _owner_id(gid, 'gid'))


# Change a symbolic link's ownership without following it.
def lchown(path, uid, gid):
    return chown(path, uid, gid, follow_symlinks=False)


class statvfs_result(tuple):
    __slots__ = ()
    n_sequence_fields = 10
    n_fields = 11
    n_unnamed_fields = 0

    # Preserve the ten sequence fields and the attribute-only filesystem id.
    def __new__(cls, sequence, dict=_stat_dict_missing):
        if dict is not _stat_dict_missing and not isinstance(dict, type({})):
            raise TypeError('os.statvfs_result() takes a dict as second arg, if any')
        values = tuple(sequence)
        if len(values) < 10:
            raise TypeError('os.statvfs_result() takes an at least 10-sequence (' + str(len(values)) + '-sequence given)')
        if len(values) > 11:
            raise TypeError('os.statvfs_result() takes an at most 11-sequence (' + str(len(values)) + '-sequence given)')
        fsid = values[10] if len(values) > 10 else (None if dict is _stat_dict_missing else dict.get('f_fsid'))
        return _host('structseq_new', cls, values[:10], (fsid,))

    f_bsize = property(lambda self: self[0])
    f_frsize = property(lambda self: self[1])
    f_blocks = property(lambda self: self[2])
    f_bfree = property(lambda self: self[3])
    f_bavail = property(lambda self: self[4])
    f_files = property(lambda self: self[5])
    f_ffree = property(lambda self: self[6])
    f_favail = property(lambda self: self[7])
    f_flag = property(lambda self: self[8])
    f_namemax = property(lambda self: self[9])
    f_fsid = property(lambda self: _host('structseq_get', self, 0))

    # Render the public fields in the struct-sequence representation.
    def __repr__(self):
        names = ('f_bsize', 'f_frsize', 'f_blocks', 'f_bfree', 'f_bavail', 'f_files', 'f_ffree', 'f_favail', 'f_flag', 'f_namemax')
        return 'os.statvfs_result(' + ', '.join(name + '=' + repr(value) for name, value in zip(names, self)) + ')'

    # Retain the attribute-only field when reducing the result.
    def __reduce__(self):
        return (type(self), (tuple(self), {'f_fsid': self.f_fsid}))

    # Refuse subclassing as CPython's native result does.
    def __init_subclass__(cls, **kwargs):
        raise TypeError("type 'os.statvfs_result' is not an acceptable base type")

statvfs_result.__module__ = 'os'


# Read filesystem capacity from a path or an open descriptor.
def statvfs(path):
    if _is_fd_path(path):
        return fstatvfs(_descriptor(path))
    original = _path(path)
    return statvfs_result(_call('statvfs', original, filename=original))


# Read filesystem capacity from an open descriptor.
def fstatvfs(fd):
    return statvfs_result(_call('fstatvfs', _cint(fd)))

# Keep native module functions callable without Python function binding.
class _BuiltinFunction(staticmethod):
    __hash__ = object.__hash__
    # Native forwarding avoids another Python call frame; class access keeps identity.
    def __get__(self, instance, owner=None):
        return self

    def __repr__(self):
        return '<built-in function ' + self.__name__ + '>'


for _name, _entry in list(globals().items()):
    if not _name.startswith('_') and callable(_entry) and not isinstance(_entry, type):
        globals()[_name] = _BuiltinFunction(_entry)
del _name, _entry
