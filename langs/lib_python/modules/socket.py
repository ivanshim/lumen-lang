# Minimal native socket adapter; CPython v3.14.8 Modules/socketmodule.c.
# Copyright (c) Python Software Foundation; PSF License in tests/python/LICENSE.
from posix import _call, _path, close as _close, dup
AF_UNIX = 1
AF_INET = 2
AF_INET6 = 10
SOCK_STREAM = 1
SOCK_DGRAM = 2

class _Socket:
    def __init__(self, family=AF_INET, type=SOCK_STREAM, proto=0, fileno=None):
        self.family = family
        self.type = type
        self.proto = proto
        self._fd = _call('socket', family, type, proto) if fileno is None else fileno
    def fileno(self):
        return self._fd
    def bind(self, address):
        if self.family != AF_UNIX:
            raise NotImplementedError('only AF_UNIX bind is available')
        _call('bind_unix', self._fd, _path(address))
    def close(self):
        fd = self._fd
        if fd != -1:
            self._fd = -1
            _close(fd)
    def __enter__(self):
        return self
    def __exit__(self, *args):
        self.close()

# The native handle implementation is the base of the public socket wrapper.
_Socket.__name__ = 'socket'
_Socket.__qualname__ = 'socket'
_Socket.__module__ = '_socket'
SocketType = _Socket

class socket(SocketType):
    pass

def fromfd(fd, family, type, proto=0):
    """Create a socket from a duplicate of the supplied descriptor."""
    nfd = dup(fd)
    return socket(family, type, proto, nfd)
