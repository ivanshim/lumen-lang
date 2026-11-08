"""The file-control calls the library and the socket tests use."""
import errno
from posix import (
    O_NONBLOCK, _call, get_blocking, get_inheritable, set_blocking, set_inheritable,
)

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

# libc lockf(3) commands. Python LOCK_* values are a different numbering.
_F_ULOCK = 0
_F_LOCK = 1
_F_TLOCK = 2


def _descriptor(fd):
    if type(fd) is int:
        return fd
    return fd.fileno()


def fcntl(fd, cmd, arg=0):
    fd = _descriptor(fd)
    if cmd == F_GETFD:
        return 0 if get_inheritable(fd) else FD_CLOEXEC
    if cmd == F_SETFD:
        set_inheritable(fd, not (arg & FD_CLOEXEC))
        return 0
    if cmd == F_GETFL:
        return 0 if get_blocking(fd) else O_NONBLOCK
    if cmd == F_SETFL:
        set_blocking(fd, not (arg & O_NONBLOCK))
        return 0
    raise OSError(errno.EINVAL, "Invalid argument")


def lockf(fd, cmd, len=0, start=0, whence=0):
    # shortcut: start and whence are ignored, upgrade when a caller locks a range
    fd = _descriptor(fd)
    if cmd & LOCK_UN:
        action = _F_ULOCK
    elif cmd & LOCK_NB:
        action = _F_TLOCK
    else:
        action = _F_LOCK
    _call("lockf", fd, action, len)
