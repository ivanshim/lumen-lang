"""The file-control calls the library and the socket tests use."""
import operator

F_DUPFD = 0
F_GETFD = 1
F_SETFD = 2
F_GETFL = 3
F_SETFL = 4
FD_CLOEXEC = 1
LOCK_SH = 1
LOCK_EX = 2
LOCK_NB = 4
LOCK_UN = 8

# Record-lock kinds for the host call. A shared bit wins when both are set.
_SHARED = 0
_EXCLUSIVE = 1
_UNLOCK = 2
_BUFFER_LIMIT = 1024


def _descriptor(fd):
    if not isinstance(fd, int):
        fileno = getattr(fd, "fileno", None)
        if fileno is None:
            raise TypeError("argument must be an int, or have a fileno() method")
        fd = fileno()
        if not isinstance(fd, int):
            raise TypeError("fileno() returned a non-integer")
    fd = _c_int(fd)
    if fd < 0:
        raise ValueError("file descriptor cannot be a negative integer")
    return fd


def _c_int(value):
    number = value if type(value) is int else operator.index(value)
    if number < -2147483648 or number > 2147483647:
        raise OverflowError("Python int too large to convert to C int")
    return number


def _index(value):
    return value if type(value) is int else operator.index(value)


def fcntl(fd, cmd, arg=0):
    from posix import _call
    fd = _descriptor(fd)
    cmd = _c_int(cmd)
    if isinstance(arg, str):
        raw = arg.encode("utf-8")
    else:
        try:
            view = memoryview(arg)
        except TypeError:
            number = _index(arg)
            if number < -9223372036854775808 or number > 9223372036854775807:
                raise OverflowError("Python int too large to convert to C long")
            return _call("fcntl", fd, cmd, number)
        if not view.c_contiguous:
            raise BufferError("memoryview: underlying buffer is not C-contiguous")
        raw = view.tobytes()
    if len(raw) > _BUFFER_LIMIT:
        raise ValueError("fcntl string arg too long")
    return _call("fcntl_buf", fd, cmd, raw)


def lockf(fd, cmd, len=0, start=0, whence=0):
    # Unlock is only the unlock command alone. Any other mix needs a shared
    # or exclusive bit; the shared bit wins when both are present.
    from posix import _call
    fd = _descriptor(fd)
    cmd = _index(cmd)
    length = _index(len)
    origin = _index(start)
    from_where = _c_int(whence)
    if cmd == LOCK_UN:
        kind = _UNLOCK
        waiting = 1
    elif cmd & LOCK_SH:
        kind = _SHARED
        waiting = 0 if cmd & LOCK_NB else 1
    elif cmd & LOCK_EX:
        kind = _EXCLUSIVE
        waiting = 0 if cmd & LOCK_NB else 1
    else:
        raise ValueError("unrecognized lockf argument")
    _call("lockf", fd, waiting, kind, from_where, origin, length)
