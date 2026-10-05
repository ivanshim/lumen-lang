# Minimal native socket adapter; CPython v3.14.8 Modules/socketmodule.c.
# Copyright (c) Python Software Foundation; PSF License in tests/python/LICENSE.
from posix import _call, _path, close as _close
AF_UNIX = 1
AF_INET = 2
AF_INET6 = 10
SOCK_STREAM = 1
SOCK_DGRAM = 2

class socket:
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
# A partial socket module: it carries the faults the module raises, and
# nothing that opens a connection. What a program asks of a real socket
# is absent, so hasattr says no rather than a wrong answer.
error = OSError

class gaierror(OSError):
    pass

class herror(OSError):
    pass

timeout = TimeoutError
