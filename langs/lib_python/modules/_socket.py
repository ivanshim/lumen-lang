# Native socket facilities for the supported socket module; CPython v3.14.8 Modules/socketmodule.c.
# Copyright (c) Python Software Foundation; PSF License in tests/python/LICENSE.
# Only descriptor creation and AF_UNIX bind reach the system. A connection, a
# name lookup beyond /etc/hosts and byte transfer are not provided: they raise
# OSError (or gaierror) instead of pretending.
import posix
from posix import _call, _path, close as _close, dup

AF_UNSPEC = 0
AF_UNIX = 1
AF_INET = 2
AF_INET6 = 10
SOCK_STREAM = 1
SOCK_DGRAM = 2
SOCK_RAW = 3
SOL_SOCKET = 1
SO_REUSEADDR = 2
SO_KEEPALIVE = 9
SOMAXCONN = 4096
IPPROTO_IP = 0
IPPROTO_TCP = 6
IPPROTO_UDP = 17
MSG_PEEK = 2
MSG_WAITALL = 256
AI_PASSIVE = 1
AI_CANONNAME = 2
has_ipv6 = False
error = OSError

class gaierror(OSError):
    pass

class herror(OSError):
    pass

timeout = TimeoutError
_default_timeout = None

def _unavailable(what):
    return OSError(95, what + ' needs network access, which is not available')

class socket:
    __slots__ = ('_family', '_type', '_proto', '_fd', '_timeout')
    def __init__(self, family=-1, type=-1, proto=-1, fileno=None):
        if fileno is None:
            if family == -1:
                family = AF_INET
            if type == -1:
                type = SOCK_STREAM
            if proto == -1:
                proto = 0
            fileno = _call('socket', int(family), int(type), int(proto))
        self._family = int(family)
        self._type = int(type)
        self._proto = int(proto)
        self._fd = fileno
        self._timeout = _default_timeout
    @property
    def family(self):
        return self._family
    @property
    def type(self):
        return self._type
    @property
    def proto(self):
        return self._proto
    @property
    def timeout(self):
        return self._timeout
    def fileno(self):
        return self._fd
    def detach(self):
        fd = self._fd
        self._fd = -1
        return fd
    def bind(self, address):
        if self._family != AF_UNIX:
            raise _unavailable('bind on an internet socket')
        _call('bind_unix', self._fd, _path(address))
    def connect(self, address):
        raise _unavailable('connect')
    def connect_ex(self, address):
        return 95
    def listen(self, backlog=0):
        raise _unavailable('listen')
    def accept(self):
        raise _unavailable('accept')
    def settimeout(self, value):
        if value is not None:
            if value < 0:
                raise ValueError('Timeout value out of range')
            value = float(value)
        self._timeout = value
    def gettimeout(self):
        return self._timeout
    def setblocking(self, flag):
        self._timeout = None if flag else 0.0
    def getblocking(self):
        return self._timeout != 0.0
    def setsockopt(self, level, name, value, optlen=None):
        if self._fd == -1:
            raise OSError(9, 'Bad file descriptor')
        raise OSError(95, 'setsockopt is not supported')
    def close(self):
        fd = self._fd
        if fd != -1:
            self._fd = -1
            _close(fd)
    def __del__(self):
        pass

socket.__name__ = 'socket'
socket.__qualname__ = 'socket'
socket.__module__ = '_socket'
SocketType = socket

def getdefaulttimeout():
    return _default_timeout

def setdefaulttimeout(value):
    global _default_timeout
    if value is not None:
        if value < 0:
            raise ValueError('Timeout value out of range')
        value = float(value)
    _default_timeout = value

def gethostname():
    return posix.uname().nodename

def _hosts():
    found = []
    try:
        with open('/etc/hosts') as f:
            for line in f:
                words = line.split('#')[0].split()
                if len(words) > 1:
                    found.append(words)
    except OSError:
        pass
    return found

def _dotted(name):
    parts = name.split('.')
    return len(parts) == 4 and all(p.isdigit() and int(p) < 256 for p in parts)

def gethostbyname(name):
    if _dotted(name):
        return name
    for words in _hosts():
        if name in words[1:] and _dotted(words[0]):
            return words[0]
    raise gaierror(-2, 'Name or service not known')

def gethostbyname_ex(name):
    address = gethostbyname(name)
    return (name, [], [address])

def gethostbyaddr(address):
    for words in _hosts():
        if words[0] == address:
            return (words[1], words[2:], [address])
    raise herror(1, 'Unknown host')

def getaddrinfo(host, port, family=0, type=0, proto=0, flags=0):
    if family not in (AF_UNSPEC, AF_INET, AF_INET6):
        raise gaierror(-6, 'ai_family not supported')
    if isinstance(host, bytes):
        host = host.decode()
    if isinstance(port, bytes):
        port = port.decode()
    if port is None:
        port = 0
    elif isinstance(port, str):
        if not port.isdigit():
            raise gaierror(-8, 'Servname not supported for ai_socktype')
        port = int(port)
    wanted = (AF_INET, AF_INET6) if family == AF_UNSPEC else (family,)
    found = []
    if host is None:
        passive = flags & AI_PASSIVE
        names = {AF_INET: '0.0.0.0' if passive else '127.0.0.1', AF_INET6: '::' if passive else '::1'}
        order = (AF_INET, AF_INET6) if passive else (AF_INET6, AF_INET)
        found = [(f, names[f]) for f in order if f in wanted]
    elif host == '' or ':' in host:
        # An IPv6 literal is taken as written; nothing resolves it further.
        if ':' in host and AF_INET6 in wanted:
            found = [(AF_INET6, host)]
    elif AF_INET in wanted:
        found = [(AF_INET, gethostbyname(host))]
    elif _dotted(host):
        raise gaierror(-9, 'Address family for hostname not supported')
    if not found:
        raise gaierror(-2, 'Name or service not known')
    numbers = {SOCK_STREAM: IPPROTO_TCP, SOCK_DGRAM: IPPROTO_UDP, SOCK_RAW: 0}
    kinds = [(type, numbers.get(type, 0))] if type else list(numbers.items())
    records = []
    for f, address in found:
        for kind, number in kinds:
            where = (address, port) if f == AF_INET else (address, port, 0, 0)
            records.append((f, kind, proto or number, '', where))
    return records

def htons(x):
    return ((x & 0xff) << 8) | ((x >> 8) & 0xff)

ntohs = htons

def htonl(x):
    return int.from_bytes(int(x).to_bytes(4, 'little'), 'big')

ntohl = htonl

def inet_aton(text):
    if not _dotted(text):
        raise OSError('illegal IP address string passed to inet_aton')
    return bytes(int(p) for p in text.split('.'))

def inet_ntoa(packed):
    if len(packed) != 4:
        raise OSError('packed IP wrong length for inet_ntoa')
    return '.'.join(str(b) for b in packed)

def socketpair(family=AF_UNIX, type=SOCK_STREAM, proto=0):
    raise _unavailable('socketpair')
