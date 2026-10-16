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
        self._timeout = _default_timeout
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

    def detach(self):
        fd = self._fd
        self._fd = -1
        return fd

    def connect(self, address):
        raise _unavailable('connect')

    def connect_ex(self, address):
        return 95

    def listen(self, backlog=0):
        raise _unavailable('listen')

    def accept(self):
        raise _unavailable('accept')

    def setsockopt(self, level, name, value, optlen=None):
        if self._fd == -1:
            raise OSError(9, 'Bad file descriptor')
        raise OSError(95, 'setsockopt is not supported')


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

# Resolver validation and metadata combined from the reviewed urllib branch.
SOMAXCONN = 4096

IPPROTO_IP = 0

MSG_PEEK = 2

MSG_WAITALL = 256

has_ipv6 = False

_default_timeout = None

def _unavailable(what):
    return OSError(95, what + ' needs network access, which is not available')

def getdefaulttimeout():
    return _default_timeout

def setdefaulttimeout(value):
    global _default_timeout
    if value is not None:
        if value < 0:
            raise ValueError('Timeout value out of range')
        value = float(value)
    _default_timeout = value

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

def _ipv6(host):
    from ipaddress import IPv6Address
    parts = host.split('%')
    scope = 0
    if len(parts) > 1:
        # shortcut: interface-name scopes need native interface lookup; reject until provided.
        if len(parts) != 2 or not parts[1] or any(c not in '0123456789' for c in parts[1]):
            raise gaierror(-2, 'Name or service not known')
        digits = parts[1].lstrip('0') or '0'
        if len(digits) > 10:
            raise gaierror(-2, 'Name or service not known')
        scope = int(digits)
        if scope > 4294967295:
            raise gaierror(-2, 'Name or service not known')
    try:
        address = str(IPv6Address(parts[0]))
    except ValueError:
        raise gaierror(-2, 'Name or service not known') from None
    return address, scope

def getaddrinfo(host, port, family=0, type=0, proto=0, flags=0):
    if flags & ~AI_PASSIVE:
        raise gaierror(-1, 'Bad value for ai_flags')
    if type not in (0, SOCK_STREAM, SOCK_DGRAM, SOCK_RAW):
        raise gaierror(-7, 'ai_socktype not supported')
    if type in (SOCK_STREAM, SOCK_DGRAM):
        expected = IPPROTO_TCP if type == SOCK_STREAM else IPPROTO_UDP
        if proto not in (0, expected):
            raise gaierror(-7, 'ai_socktype not supported')
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
    if port and (type == SOCK_RAW or type == 0 and proto not in (0, IPPROTO_TCP, IPPROTO_UDP)):
        raise gaierror(-8, 'Servname not supported for ai_socktype')
    wanted = (AF_INET, AF_INET6) if family == AF_UNSPEC else (family,)
    scope = 0
    found = []
    if host is None:
        passive = flags & AI_PASSIVE
        names = {AF_INET: '0.0.0.0' if passive else '127.0.0.1', AF_INET6: '::' if passive else '::1'}
        order = (AF_INET, AF_INET6) if passive else (AF_INET6, AF_INET)
        found = [(f, names[f]) for f in order if f in wanted]
    elif ':' in host:
        address, scope = _ipv6(host)
        if AF_INET6 not in wanted:
            raise gaierror(-9, 'Address family for hostname not supported')
        found = [(AF_INET6, address)]
    elif host == '':
        raise gaierror(-2, 'Name or service not known')
    elif AF_INET in wanted:
        found = [(AF_INET, gethostbyname(host))]
    elif _dotted(host):
        raise gaierror(-9, 'Address family for hostname not supported')
    if not found:
        raise gaierror(-2, 'Name or service not known')
    numbers = {SOCK_STREAM: IPPROTO_TCP, SOCK_DGRAM: IPPROTO_UDP, SOCK_RAW: 0}
    if type:
        kinds = [(type, proto or numbers[type])]
    elif proto and proto not in (IPPROTO_TCP, IPPROTO_UDP):
        kinds = [(SOCK_RAW, proto)]
    else:
        kinds = [(kind, number) for kind, number in numbers.items() if not proto or number == proto]
    records = []
    for f, address in found:
        for kind, number in kinds:
            where = (address, port) if f == AF_INET else (address, port, 0, scope)
            records.append((f, kind, number, '', where))
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


# From CPython v3.14.8 Lib/socket.py; PSF License.
def create_connection(address, timeout=_GLOBAL_DEFAULT_TIMEOUT,
                      source_address=None, *, all_errors=False):
    """Connect to *address* and return the socket object.

    Convenience function.  Connect to *address* (a 2-tuple ``(host,
    port)``) and return the socket object.  Passing the optional
    *timeout* parameter will set the timeout on the socket instance
    before attempting to connect.  If no *timeout* is supplied, the
    global default timeout setting returned by :func:`getdefaulttimeout`
    is used.  If *source_address* is set it must be a tuple of (host, port)
    for the socket to bind as a source address before making the connection.
    A host of '' or port 0 tells the OS to use the default. When a connection
    cannot be created, raises the last error if *all_errors* is False,
    and an ExceptionGroup of all errors if *all_errors* is True.
    """

    host, port = address
    exceptions = []
    for res in getaddrinfo(host, port, 0, SOCK_STREAM):
        af, socktype, proto, canonname, sa = res
        sock = None
        try:
            sock = socket(af, socktype, proto)
            if timeout is not _GLOBAL_DEFAULT_TIMEOUT:
                sock.settimeout(timeout)
            if source_address:
                sock.bind(source_address)
            sock.connect(sa)
            # Break explicitly a reference cycle
            exceptions.clear()
            return sock

        except error as exc:
            if not all_errors:
                exceptions.clear()  # raise only the last error
            exceptions.append(exc)
            if sock is not None:
                sock.close()

    if len(exceptions):
        try:
            if not all_errors:
                raise exceptions[0]
            raise ExceptionGroup("create_connection failed", exceptions)
        finally:
            # Break explicitly a reference cycle
            exceptions.clear()
    else:
        raise error("getaddrinfo returns an empty list")

