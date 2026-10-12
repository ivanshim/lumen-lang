# Minimal native socket adapter; CPython v3.14.8 Modules/socketmodule.c.
# Copyright (c) Python Software Foundation; PSF License in tests/python/LICENSE.
from posix import _call, _path, close as _close, dup, read as _read, write as _write
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
    def setblocking(self, flag):
        _call('set_blocking', self._fd, bool(flag))
        self._timeout = None if flag else 0.0
    def getblocking(self):
        return getattr(self, '_timeout', None) != 0.0
    def settimeout(self, timeout):
        if timeout is not None and timeout < 0:
            raise ValueError('Timeout value out of range')
        self.setblocking(timeout is None)
        self._timeout = timeout
    def gettimeout(self):
        return getattr(self, '_timeout', None)
    def recv(self, buffersize, flags=0):
        if flags:
            raise NotImplementedError('recv flags are unavailable')
        return _read(self._fd, buffersize)
    def send(self, data, flags=0):
        if flags:
            raise NotImplementedError('send flags are unavailable')
        return _write(self._fd, data)
    def shutdown(self, how):
        _call('socket_shutdown', self._fd, how)
    def __del__(self):
        self.close()
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
error = OSError

class gaierror(OSError):
    pass

class herror(OSError):
    pass

timeout = TimeoutError


# An omitted timeout is distinct from None, which requests blocking I/O.
_GLOBAL_DEFAULT_TIMEOUT = object()

# Read the operating system hostname through the existing Python host bridge.
def gethostname():
    return _call('socket_hostname').decode('utf-8', 'surrogateescape')

# Resolve a real address and preserve the resolver's exception category.
def gethostbyaddr(ip_address):
    if isinstance(ip_address, str):
        address = ip_address.encode('idna')
    elif isinstance(ip_address, (bytes, bytearray)):
        address = bytes(ip_address)
    else:
        raise TypeError('gethostbyaddr() argument 1 must be str, bytes or bytearray, not ' + type(ip_address).__name__)
    if b'\0' in address:
        raise TypeError('gethostbyaddr() argument 1 must be encoded string without null bytes, not ' + type(ip_address).__name__)
    kind, code, message, result = _call('socket_hostbyaddr', address)
    if kind == 1:
        raise gaierror(code, message.decode('utf-8', 'surrogateescape'))
    if kind == 2:
        raise herror(code, message.decode('utf-8', 'surrogateescape'))
    hostname, aliases, addresses = result
    return (hostname.decode('utf-8', 'surrogateescape'),
            [name.decode('utf-8', 'surrogateescape') for name in aliases],
            [name.decode('ascii') for name in addresses])

# From CPython v3.14.8 (8e6e75d9102e), Lib/socket.py: getfqdn; PSF License.
def getfqdn(name=''):
    """Get fully qualified domain name from name.

    An empty argument is interpreted as meaning the local host.

    First the hostname returned by gethostbyaddr() is checked, then
    possibly existing aliases. In case no FQDN is available and `name`
    was given, it is returned unchanged. If `name` was empty, '0.0.0.0' or '::',
    hostname from gethostname() is returned.
    """
    name = name.strip()
    if not name or name in ('0.0.0.0', '::'):
        name = gethostname()
    try:
        hostname, aliases, ipaddrs = gethostbyaddr(name)
    except error:
        pass
    else:
        aliases.insert(0, hostname)
        for name in aliases:
            if '.' in name:
                break
        else:
            name = hostname
    return name
def socketpair(family=AF_UNIX, type=SOCK_STREAM, proto=0):
    first, second = _call('socketpair', family, type, proto)
    try:
        left = socket(family, type, proto, first)
        right = socket(family, type, proto, second)
    except BaseException:
        _close(first)
        _close(second)
        raise
    return left, right

SHUT_RD = 0
SHUT_WR = 1
SHUT_RDWR = 2

# Linux socket ABI constants exposed by the host socket interface.
AF_UNSPEC = 0
AI_PASSIVE = 1
AI_CANONNAME = 2
AI_NUMERICHOST = 4
AI_NUMERICSERV = 1024
AI_ADDRCONFIG = 32
AI_V4MAPPED = 8
AI_ALL = 16
SOL_SOCKET = 1
SO_REUSEADDR = 2
SO_ERROR = 4
SO_KEEPALIVE = 9
SO_REUSEPORT = 15
IPPROTO_TCP = 6
IPPROTO_UDP = 17
TCP_NODELAY = 1
SOCK_RAW = 3
