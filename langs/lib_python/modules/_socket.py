# Native socket handles for CPython v3.14.8 Modules/socketmodule.c. This
# module packs every address into its sockaddr bytes and keeps each handle's
# timeout; the host bridge makes the system calls on bare descriptors.
# Copyright (c) Python Software Foundation; PSF License in tests/python/LICENSE.
import sys as _sys
import time as _time
from math import ceil as _ceil
from _operator import index as _index
from posix import _call, close as _close, strerror as _strerror

# Linux socket ABI constants, as the host socket interface defines them.
AF_UNSPEC = 0
AF_UNIX = 1
AF_INET = 2
AF_INET6 = 10
SOCK_STREAM = 1
SOCK_DGRAM = 2
SOCK_RAW = 3
SOCK_RDM = 4
SOCK_SEQPACKET = 5
SOCK_NONBLOCK = 2048
SOCK_CLOEXEC = 524288
SOL_SOCKET = 1
SOL_IP = 0
SOL_TCP = 6
SOL_UDP = 17
SOL_IPV6 = 41
SO_DEBUG = 1
SO_REUSEADDR = 2
SO_TYPE = 3
SO_ERROR = 4
SO_DONTROUTE = 5
SO_BROADCAST = 6
SO_SNDBUF = 7
SO_RCVBUF = 8
SO_KEEPALIVE = 9
SO_OOBINLINE = 10
SO_LINGER = 13
SO_REUSEPORT = 15
SO_RCVLOWAT = 18
SO_SNDLOWAT = 19
SO_RCVTIMEO = 20
SO_SNDTIMEO = 21
SO_BINDTODEVICE = 25
SO_ACCEPTCONN = 30
SO_PROTOCOL = 38
SO_DOMAIN = 39
SOMAXCONN = 4096
MSG_OOB = 1
MSG_PEEK = 2
MSG_DONTROUTE = 4
MSG_CTRUNC = 8
MSG_TRUNC = 32
MSG_DONTWAIT = 64
MSG_EOR = 128
MSG_WAITALL = 256
MSG_CONFIRM = 2048
MSG_ERRQUEUE = 8192
MSG_NOSIGNAL = 16384
MSG_MORE = 32768
MSG_CMSG_CLOEXEC = 1073741824
IPPROTO_IP = 0
IPPROTO_ICMP = 1
IPPROTO_IGMP = 2
IPPROTO_IPIP = 4
IPPROTO_TCP = 6
IPPROTO_EGP = 8
IPPROTO_PUP = 12
IPPROTO_UDP = 17
IPPROTO_IDP = 22
IPPROTO_TP = 29
IPPROTO_IPV6 = 41
IPPROTO_ROUTING = 43
IPPROTO_FRAGMENT = 44
IPPROTO_RSVP = 46
IPPROTO_GRE = 47
IPPROTO_ESP = 50
IPPROTO_AH = 51
IPPROTO_ICMPV6 = 58
IPPROTO_NONE = 59
IPPROTO_DSTOPTS = 60
IPPROTO_PIM = 103
IPPROTO_SCTP = 132
IPPROTO_UDPLITE = 136
IPPROTO_RAW = 255
IP_TOS = 1
IP_TTL = 2
IP_HDRINCL = 3
IP_OPTIONS = 4
IP_RECVOPTS = 6
IP_RETOPTS = 7
IP_RECVTOS = 13
IP_MULTICAST_IF = 32
IP_MULTICAST_TTL = 33
IP_MULTICAST_LOOP = 34
IP_ADD_MEMBERSHIP = 35
IP_DROP_MEMBERSHIP = 36
IPV6_CHECKSUM = 7
IPV6_UNICAST_HOPS = 16
IPV6_MULTICAST_IF = 17
IPV6_MULTICAST_HOPS = 18
IPV6_MULTICAST_LOOP = 19
IPV6_JOIN_GROUP = 20
IPV6_LEAVE_GROUP = 21
IPV6_V6ONLY = 26
IPV6_RECVPKTINFO = 49
IPV6_PKTINFO = 50
IPV6_RECVHOPLIMIT = 51
IPV6_HOPLIMIT = 52
IPV6_RECVTCLASS = 66
IPV6_TCLASS = 67
TCP_NODELAY = 1
TCP_MAXSEG = 2
TCP_CORK = 3
TCP_KEEPIDLE = 4
TCP_KEEPINTVL = 5
TCP_KEEPCNT = 6
TCP_SYNCNT = 7
TCP_LINGER2 = 8
TCP_DEFER_ACCEPT = 9
TCP_WINDOW_CLAMP = 10
TCP_INFO = 11
TCP_QUICKACK = 12
TCP_CONGESTION = 13
TCP_USER_TIMEOUT = 18
TCP_FASTOPEN = 23
TCP_NOTSENT_LOWAT = 25
SHUT_RD = 0
SHUT_WR = 1
SHUT_RDWR = 2
AI_PASSIVE = 1
AI_CANONNAME = 2
AI_NUMERICHOST = 4
AI_V4MAPPED = 8
AI_ALL = 16
AI_ADDRCONFIG = 32
AI_NUMERICSERV = 1024
NI_NUMERICHOST = 1
NI_NUMERICSERV = 2
NI_NOFQDN = 4
NI_NAMEREQD = 8
NI_DGRAM = 16
NI_MAXHOST = 1025
NI_MAXSERV = 32
EAI_BADFLAGS = -1
EAI_NONAME = -2
EAI_AGAIN = -3
EAI_FAIL = -4
EAI_NODATA = -5
EAI_FAMILY = -6
EAI_SOCKTYPE = -7
EAI_SERVICE = -8
EAI_ADDRFAMILY = -9
EAI_MEMORY = -10
EAI_SYSTEM = -11
EAI_OVERFLOW = -12
INADDR_ANY = 0
INADDR_LOOPBACK = 0x7f000001
INADDR_BROADCAST = 0xffffffff
INADDR_NONE = 0xffffffff
INADDR_UNSPEC_GROUP = 0xe0000000
INADDR_ALLHOSTS_GROUP = 0xe0000001
INADDR_MAX_LOCAL_GROUP = 0xe00000ff
has_ipv6 = True

error = OSError

class gaierror(OSError):
    pass

class herror(OSError):
    pass

timeout = TimeoutError

_EINTR = 4
_EAGAIN = 11
_EAFNOSUPPORT = 97
_EINPROGRESS = 115
_EISCONN = 106
_POLLIN = 1
_POLLOUT = 4
_defaulttimeout = None


def _parse_timeout(value):
    # socket_parse_timeout: None blocks; seconds are kept as a float; a
    # negative amount or NaN is out of range.
    if value is None:
        return None
    if isinstance(value, float):
        seconds = float(value)
        if seconds != seconds:
            raise ValueError('Invalid value NaN (not a number)')
    else:
        seconds = float(_index(value))
    if seconds > 9223372036.0:
        raise OverflowError('timestamp too large to convert to C PyTime_t')
    if seconds < 0:
        raise ValueError('Timeout value out of range')
    return seconds


def _buffer(data):
    try:
        return memoryview(data)
    except TypeError:
        raise TypeError("a bytes-like object is required, not '%s'" % type(data).__name__) from None


def _as_bytes(data):
    # The bytes of a bytes-like object. Plain bytes pass through untouched:
    # no temporary view is made for them, so a pending signal taken up while
    # a temporary is finalized cannot swallow its handler's exception.
    if isinstance(data, bytes):
        return data
    return _buffer(data).tobytes()


def _host_bytes(host):
    # idna_converter: text is encoded with the IDNA codec, bytes pass through.
    if isinstance(host, str):
        name = host.encode('idna')
    elif isinstance(host, (bytes, bytearray)):
        name = bytes(host)
    else:
        raise TypeError('str, bytes or bytearray expected, not %s' % type(host).__name__)
    if b'\0' in name:
        raise TypeError('host name must not contain null character')
    return name


def _family_bytes(family):
    return family.to_bytes(2, _sys.byteorder)


def _resolve_host(host, family):
    # setipaddr: '' is the wildcard address, '<broadcast>' the IPv4 broadcast
    # address, numeric text is read directly and anything else is looked up.
    # The answer is a whole sockaddr with the port left zero.
    name = _host_bytes(host)
    if name == b'':
        if family == AF_INET6:
            return _family_bytes(AF_INET6) + bytes(26)
        return _family_bytes(AF_INET) + bytes(14)
    if name == b'<broadcast>':
        if family not in (AF_INET, AF_UNSPEC):
            raise OSError('address family mismatched')
        return _family_bytes(AF_INET) + bytes(2) + b'\xff\xff\xff\xff' + bytes(8)
    if family in (AF_INET, AF_UNSPEC):
        packed = _call('inet_packed', AF_INET, name)
        if packed is not None:
            return _family_bytes(AF_INET) + bytes(2) + packed + bytes(8)
    if family in (AF_INET6, AF_UNSPEC):
        packed = _call('inet_packed', AF_INET6, name)
        if packed is not None:
            return _family_bytes(AF_INET6) + bytes(6) + packed + bytes(4)
    code, found = _call('getaddrinfo', name, None, family, 0, 0, 0)
    if code:
        raise gaierror(code, found.decode('utf-8', 'surrogateescape'))
    return found[0][4]


def _pack_address(family, address, caller):
    # getsockaddrarg: an address tuple or path becomes the family's sockaddr.
    if family == AF_UNIX:
        if isinstance(address, str):
            path = address.encode('utf-8', 'surrogateescape')
        else:
            path = _as_bytes(address)
        # Linux abstract namespace: an empty or NUL-led name is bounded by
        # the length; a pathname needs room for its terminator.
        if path[:1] in (b'', b'\0'):
            if len(path) > 108:
                raise OSError('AF_UNIX path too long')
        elif len(path) >= 108:
            raise OSError('AF_UNIX path too long')
        return _family_bytes(AF_UNIX) + path
    if family == AF_INET:
        if not isinstance(address, tuple):
            raise TypeError('%s(): AF_INET address must be tuple, not %s' % (caller, type(address).__name__))
        if len(address) != 2:
            raise TypeError('AF_INET address must be a pair (host, port)')
        host, port = address
        port = _index(port)
        if not 0 <= port <= 65535:
            raise OverflowError('%s(): port must be 0-65535.' % caller)
        raw = _resolve_host(host, AF_INET)
        return raw[:2] + port.to_bytes(2, 'big') + raw[4:]
    if family == AF_INET6:
        if not isinstance(address, tuple):
            raise TypeError('%s(): AF_INET6 address must be tuple, not %s' % (caller, type(address).__name__))
        if not 2 <= len(address) <= 4:
            raise TypeError('AF_INET6 address must be a tuple (host, port[, flowinfo[, scopeid]])')
        host, port = address[:2]
        flowinfo = _index(address[2]) if len(address) > 2 else 0
        scope_id = _index(address[3]) if len(address) > 3 else 0
        port = _index(port)
        if not 0 <= port <= 65535:
            raise OverflowError('%s(): port must be 0-65535.' % caller)
        if not 0 <= flowinfo <= 0xfffff:
            raise OverflowError('%s(): flowinfo must be 0-1048575.' % caller)
        raw = _resolve_host(host, AF_INET6)
        return (raw[:2] + port.to_bytes(2, 'big') + flowinfo.to_bytes(4, 'big') + raw[8:24]
                + scope_id.to_bytes(4, _sys.byteorder))
    raise OSError('%s(): bad family' % caller)


def _unpack_address(raw):
    # makesockaddr: the sockaddr bytes back into the family's address value.
    family = int.from_bytes(raw[:2], _sys.byteorder)
    if family == AF_INET:
        return '%d.%d.%d.%d' % tuple(raw[4:8]), int.from_bytes(raw[2:4], 'big')
    if family == AF_INET6:
        return (_call('inet_text', AF_INET6, raw[8:24]), int.from_bytes(raw[2:4], 'big'),
                int.from_bytes(raw[4:8], 'big'), int.from_bytes(raw[24:28], _sys.byteorder))
    if family == AF_UNIX:
        path = raw[2:]
        if path[:1] == b'\0':
            return bytes(path)
        return path.split(b'\0', 1)[0].decode('utf-8', 'surrogateescape')
    return family, bytes(raw[2:16])


def _option_int(fd, level, name):
    return int.from_bytes(_call('sock_getopt', fd, level, name, 4), _sys.byteorder, signed=True)


class socket:
    """socket(family=AF_INET, type=SOCK_STREAM, proto=0) -> socket object
socket(family=-1, type=-1, proto=-1, fileno=None) -> socket object

Open a socket of the given type.  The family argument specifies the
address family; it defaults to AF_INET.  The type argument specifies
whether this is a stream (SOCK_STREAM, this is the default)
or datagram (SOCK_DGRAM) socket.  The protocol argument defaults to 0,
specifying the default protocol.  Keyword arguments are accepted.
The socket is created as non-inheritable.

When a fileno is passed in, family, type and proto are auto-detected,
unless they are explicitly set.

A socket object represents one endpoint of a network connection."""
    # No instance dictionary and no weak-reference slot of its own, like
    # the reference's C type: the public subclass adds those in its slots.
    __slots__ = ('_fd', '_family', '_type', '_proto', '_timeout')

    def __init__(self, family=-1, type=-1, proto=-1, fileno=None):
        family, type, proto = _index(family), _index(type), _index(proto)
        if fileno is None:
            if family == -1:
                family = AF_INET
            if type == -1:
                type = SOCK_STREAM
            if proto == -1:
                proto = 0
            fd = _call('socket', family, type, proto)
        else:
            fd = _index(fileno)
            if fd < 0:
                raise ValueError('negative file descriptor')
            if family == -1:
                family = int.from_bytes(_call('sock_name', fd, 0)[:2], _sys.byteorder)
            if type == -1:
                type = _option_int(fd, SOL_SOCKET, SO_TYPE)
            if proto == -1:
                proto = _option_int(fd, SOL_SOCKET, SO_PROTOCOL)
        self._fd = fd
        self._family = family
        self._type = type & ~(SOCK_NONBLOCK | SOCK_CLOEXEC)
        self._proto = proto
        self._timeout = 0.0 if type & SOCK_NONBLOCK else _defaulttimeout
        if self._timeout is not None:
            _call('set_blocking', fd, False)

    family = property(lambda self: self._family, doc='the socket family')
    type = property(lambda self: self._type, doc='the socket type')
    proto = property(lambda self: self._proto, doc='the socket protocol')
    timeout = property(lambda self: self._timeout, doc='the socket timeout')

    def __repr__(self):
        return '<socket object, fd=%d, family=%d, type=%d, proto=%d>' % (
            self._fd, self._family, self._type, self._proto)

    def __del__(self):
        # sock_finalize: an open handle is reported, then closed.
        fd = getattr(self, '_fd', -1)
        if fd != -1:
            # The warning shows the open handle; the descriptor is closed
            # whether or not the warning is raised as an error.
            try:
                import warnings
                warnings.warn('unclosed %r' % (self,), ResourceWarning, source=self)
            finally:
                self._fd = -1
                try:
                    _close(fd)
                except OSError:
                    pass

    def fileno(self):
        return self._fd

    def close(self):
        fd = self._fd
        if fd != -1:
            self._fd = -1
            try:
                _close(fd)
            except ConnectionResetError:
                pass

    def detach(self):
        fd = self._fd
        self._fd = -1
        return fd

    def _apply_timeout(self, seconds):
        # The descriptor itself blocks only when no timeout is set.
        _call('set_blocking', self._fd, seconds is None)
        self._timeout = seconds

    def setblocking(self, flag, /):
        self._apply_timeout(None if flag else 0.0)

    def getblocking(self):
        return self._timeout != 0.0

    def settimeout(self, value, /):
        self._apply_timeout(_parse_timeout(value))

    def gettimeout(self):
        return self._timeout

    def _deadline(self):
        timeout = self._timeout
        return None if not timeout else _time.monotonic() + timeout

    def _await(self, events, deadline):
        # Wait for the descriptor until the deadline, which raises 'timed out'.
        while True:
            left = deadline - _time.monotonic()
            if left <= 0:
                raise TimeoutError('timed out')
            try:
                ready = _call('poll', ((self._fd, events),), _ceil(left * 1000))
            except InterruptedError:
                continue
            if ready:
                return

    def _perform(self, events, deadline, op, *args):
        # sock_call_ex: with a timeout, wait for readiness first; start the
        # call over after a signal whose handler returned, and after a
        # ready descriptor still answers that it would block.
        while True:
            # A closed handle is not waited on: the call itself reports EBADF.
            if deadline is not None and self._fd != -1:
                self._await(events, deadline)
            try:
                return _call(op, self._fd, *args)
            except InterruptedError:
                continue
            except BlockingIOError:
                if deadline is None:
                    raise

    def bind(self, address, /):
        _call('sock_bind', self._fd, _pack_address(self._family, address, 'bind'))

    def listen(self, backlog=min(SOMAXCONN, 128), /):
        backlog = _index(backlog)
        _call('sock_listen', self._fd, backlog if backlog > 0 else 0)

    def _accept(self):
        fd, raw = self._perform(_POLLIN, self._deadline(), 'sock_accept')
        return fd, _unpack_address(raw)

    def _connect(self, address, caller):
        # internal_connect: a connection still in progress is awaited when
        # the socket blocks or has a timeout, then judged by SO_ERROR.
        raw = _pack_address(self._family, address, caller)
        try:
            _call('sock_connect', self._fd, raw)
            return 0
        except OSError as failure:
            err = failure.errno
        timeout = self._timeout
        if err == _EINTR:
            wait = timeout != 0.0
        else:
            wait = timeout is not None and timeout > 0 and err == _EINPROGRESS
        if wait:
            deadline = None if timeout is None else _time.monotonic() + timeout
            while True:
                if deadline is None:
                    try:
                        _call('poll', ((self._fd, _POLLOUT),), -1)
                    except InterruptedError:
                        continue
                else:
                    try:
                        self._await(_POLLOUT, deadline)
                    except TimeoutError:
                        if caller == 'connect':
                            raise
                        return _EAGAIN
                err = _option_int(self._fd, SOL_SOCKET, SO_ERROR)
                if err in (0, _EISCONN):
                    return 0
                if err != _EAGAIN or deadline is None:
                    break
        if caller == 'connect':
            raise OSError(err, _strerror(err))
        return err

    def connect(self, address, /):
        self._connect(address, 'connect')

    def connect_ex(self, address, /):
        return self._connect(address, 'connect_ex')

    def getsockname(self):
        return _unpack_address(_call('sock_name', self._fd, 0))

    def getpeername(self):
        return _unpack_address(_call('sock_name', self._fd, 1))

    def recv(self, bufsize, flags=0, /):
        if _index(bufsize) < 0:
            raise ValueError('negative buffersize in recv')
        return self._perform(_POLLIN, self._deadline(), 'sock_recv', bufsize, _index(flags))[0]

    def recvfrom(self, bufsize, flags=0, /):
        if _index(bufsize) < 0:
            raise ValueError('negative buffersize in recvfrom')
        data, raw, _ = self._perform(_POLLIN, self._deadline(), 'sock_recvfrom', bufsize, _index(flags))
        return data, (_unpack_address(raw) if raw else None)

    def _writable(self, buffer, nbytes, caller):
        view = _buffer(buffer)
        if view.readonly:
            raise TypeError('%s() argument 1 must be read-write bytes-like object, not %s'
                            % (caller, type(buffer).__name__))
        view = view.cast('B')
        nbytes = _index(nbytes)
        if nbytes < 0:
            raise ValueError('negative buffersize in %s' % caller)
        if nbytes > view.nbytes:
            raise ValueError('buffer too small for requested bytes')
        return view, (nbytes or view.nbytes)

    def recv_into(self, buffer, nbytes=0, flags=0):
        view, nbytes = self._writable(buffer, nbytes, 'recv_into')
        # The host's count, not the bytes copied: MSG_TRUNC reports a whole datagram.
        data, count = self._perform(_POLLIN, self._deadline(), 'sock_recv', nbytes, _index(flags))
        view[:len(data)] = data
        return count

    def recvfrom_into(self, buffer, nbytes=0, flags=0):
        view, nbytes = self._writable(buffer, nbytes, 'recvfrom_into')
        data, raw, count = self._perform(_POLLIN, self._deadline(), 'sock_recvfrom', nbytes, _index(flags))
        view[:len(data)] = data
        return count, (_unpack_address(raw) if raw else None)

    def send(self, data, flags=0, /):
        return self._perform(_POLLOUT, self._deadline(), 'sock_send', _as_bytes(data), _index(flags))

    def sendall(self, data, flags=0, /):
        # One deadline covers the whole transfer, and the host sends from
        # an offset into one buffer, so no temporary is made per round
        # (a pending signal would otherwise be taken up in its finalizer).
        data = _as_bytes(data)
        flags = _index(flags)
        deadline = self._deadline()
        sent = 0
        while sent < len(data):
            sent += self._perform(_POLLOUT, deadline, 'sock_send', data, flags, sent)

    def sendto(self, data, *args):
        if len(args) == 1:
            flags, address = 0, args[0]
        elif len(args) == 2:
            flags, address = args
        else:
            raise TypeError('sendto() takes 2 or 3 arguments (%d given)' % (len(args) + 1))
        data = _as_bytes(data)
        flags = _index(flags)
        raw = _pack_address(self._family, address, 'sendto')
        return self._perform(_POLLOUT, self._deadline(), 'sock_sendto', data, flags, raw)

    def shutdown(self, how, /):
        _call('socket_shutdown', self._fd, _index(how))

    def setsockopt(self, level, optname, value, /):
        if isinstance(value, int):
            value = _index(value)
            if not -2147483648 <= value <= 2147483647:
                raise OverflowError('Python int too large to convert to C int')
            data = value.to_bytes(4, _sys.byteorder, signed=True)
        else:
            data = _as_bytes(value)
        _call('sock_setopt', self._fd, _index(level), _index(optname), data)

    def getsockopt(self, level, optname, buflen=0, /):
        level, optname, buflen = _index(level), _index(optname), _index(buflen)
        if buflen == 0:
            return _option_int(self._fd, level, optname)
        if buflen < 0 or buflen > 1024:
            raise OSError('getsockopt buflen out of range')
        return _call('sock_getopt', self._fd, level, optname, buflen)

    def __enter__(self):
        return self

    def __exit__(self, *args):
        self.close()


SocketType = socket


def close(fd):
    try:
        _close(_index(fd))
    except ConnectionResetError:
        pass


def dup(fd):
    return _call('dup', _index(fd))


def socketpair(family=AF_UNIX, type=SOCK_STREAM, proto=0):
    family, type, proto = _index(family), _index(type), _index(proto)
    first, second = _call('socketpair', family, type, proto)
    try:
        return socket(family, type, proto, first), socket(family, type, proto, second)
    except BaseException:
        _close(first)
        _close(second)
        raise


def getdefaulttimeout():
    return _defaulttimeout


def setdefaulttimeout(timeout):
    global _defaulttimeout
    _defaulttimeout = _parse_timeout(timeout)


def gethostname():
    return _call('socket_hostname').decode('utf-8', 'surrogateescape')


def gethostbyname(hostname):
    return _unpack_address(_resolve_host(hostname, AF_INET))[0]


def _host_entry(answer):
    # A host database answer: a lookup error of either kind, or the entry.
    kind, code, message, result = answer
    if kind == 1:
        raise gaierror(code, message.decode('utf-8', 'surrogateescape'))
    if kind == 2:
        raise herror(code, message.decode('utf-8', 'surrogateescape'))
    hostname, aliases, addresses = result
    return (hostname.decode('utf-8', 'surrogateescape'),
            [name.decode('utf-8', 'surrogateescape') for name in aliases],
            [name.decode('ascii') for name in addresses])


def gethostbyname_ex(hostname):
    return _host_entry(_call('socket_hostbyname', _host_bytes(hostname)))


def gethostbyaddr(ip_address):
    described = 'None' if ip_address is None else type(ip_address).__name__
    if isinstance(ip_address, str):
        address = ip_address.encode('idna')
    elif isinstance(ip_address, (bytes, bytearray)):
        address = bytes(ip_address)
    else:
        raise TypeError('gethostbyaddr() argument 1 must be str, bytes or bytearray, not ' + described)
    if b'\0' in address:
        raise TypeError('gethostbyaddr() argument 1 must be encoded string without null bytes, not ' + described)
    return _host_entry(_call('socket_hostbyaddr', address))


def getaddrinfo(host, port, family=0, type=0, proto=0, flags=0):
    if host is None:
        node = None
    elif isinstance(host, str):
        node = host.encode('idna')
    elif isinstance(host, (bytes, bytearray)):
        node = bytes(host)
    else:
        raise TypeError('getaddrinfo() argument 1 must be string or None')
    if port is None:
        service = None
    elif isinstance(port, int) and not isinstance(port, bool):
        service = str(port).encode('ascii')
    elif isinstance(port, str):
        service = port.encode('utf-8')
    elif isinstance(port, bytes):
        service = port
    else:
        raise OSError('Int or String expected')
    code, found = _call('getaddrinfo', node, service, _index(family), _index(type), _index(proto), _index(flags))
    if code:
        raise gaierror(code, found.decode('utf-8', 'surrogateescape'))
    return [(fam, kind, number, canonical.decode('utf-8', 'surrogateescape'), _unpack_address(raw))
            for fam, kind, number, canonical, raw in found]


def getnameinfo(sockaddr, flags, /):
    # The host part must name exactly one numeric address, which decides
    # the family; an IPv4 address takes a pair, IPv6 up to four fields.
    if not isinstance(sockaddr, tuple) or not 2 <= len(sockaddr) <= 4:
        raise TypeError('getnameinfo() argument 1 must be a tuple')
    host, port = sockaddr[0], _index(sockaddr[1])
    flowinfo = _index(sockaddr[2]) if len(sockaddr) > 2 else 0
    scope_id = _index(sockaddr[3]) if len(sockaddr) > 3 else 0
    if not 0 <= flowinfo <= 0xfffff:
        raise OverflowError('getnameinfo(): flowinfo must be 0-1048575.')
    code, found = _call('getaddrinfo', _host_bytes(host), str(port).encode('ascii'), AF_UNSPEC, SOCK_DGRAM, 0, AI_NUMERICHOST)
    if code:
        raise gaierror(code, found.decode('utf-8', 'surrogateescape'))
    if len(found) > 1:
        raise OSError('sockaddr resolved to multiple addresses')
    family, raw = found[0][0], found[0][4]
    if family == AF_INET and len(sockaddr) != 2:
        raise OSError('IPv4 sockaddr must be 2 tuple')
    if family == AF_INET6:
        raw = raw[:4] + flowinfo.to_bytes(4, 'big') + raw[8:24] + scope_id.to_bytes(4, _sys.byteorder)
    answer = _call('getnameinfo', raw, _index(flags))
    if answer[0]:
        raise gaierror(answer[0], answer[1].decode('utf-8', 'surrogateescape'))
    return answer[1].decode('utf-8', 'surrogateescape'), answer[2].decode('utf-8', 'surrogateescape')


def _database_name(value, caller, position):
    if value is not None and not isinstance(value, str):
        raise TypeError('%s() argument %d must be str, not %s' % (caller, position, type(value).__name__))
    return value


def getservbyname(servicename, protocolname=None, /):
    port = _call('sock_db', 0, _database_name(servicename, 'getservbyname', 1),
                 _database_name(protocolname, 'getservbyname', 2))
    if port is None:
        raise OSError('service/proto not found')
    return port


def getservbyport(port, protocolname=None, /):
    port = _index(port)
    if not 0 <= port <= 65535:
        raise OverflowError('getservbyport: port must be 0-65535.')
    name = _call('sock_db', 1, port, _database_name(protocolname, 'getservbyport', 2))
    if name is None:
        raise OSError('port/proto not found')
    return name.decode('utf-8', 'surrogateescape')


def getprotobyname(name, /):
    number = _call('sock_db', 2, _database_name(name, 'getprotobyname', 1), None)
    if number is None:
        raise OSError('protocol not found')
    return number


def inet_aton(ip_addr, /):
    packed = _call('inet_packed', 0, _database_name(ip_addr, 'inet_aton', 1))
    if packed is None:
        raise OSError('illegal IP address string passed to inet_aton')
    return packed


def inet_ntoa(packed_ip, /):
    data = _as_bytes(packed_ip)
    if len(data) != 4:
        raise OSError('packed IP wrong length for inet_ntoa')
    return '%d.%d.%d.%d' % tuple(data)


def inet_pton(address_family, ip_string, /):
    address_family = _index(address_family)
    if address_family not in (AF_INET, AF_INET6):
        raise OSError(_EAFNOSUPPORT, _strerror(_EAFNOSUPPORT))
    packed = _call('inet_packed', address_family, _database_name(ip_string, 'inet_pton', 2))
    if packed is None:
        raise OSError('illegal IP address string passed to inet_pton')
    return packed


def inet_ntop(address_family, packed_ip, /):
    address_family = _index(address_family)
    data = _as_bytes(packed_ip)
    if address_family == AF_INET:
        expected = 4
    elif address_family == AF_INET6:
        expected = 16
    else:
        raise ValueError('unknown address family %d' % address_family)
    if len(data) != expected:
        raise ValueError('invalid length of packed IP address string')
    return _call('inet_text', address_family, data)


def _byte_order(value, bits):
    # The uint16_t and uint32_t converters of the reference.
    value = _index(value)
    if value < 0:
        raise ValueError('Cannot convert negative int')
    if value >> bits:
        raise OverflowError('Python int too large for C uint%d_t' % bits)
    if _sys.byteorder == 'big':
        return value
    return int.from_bytes(value.to_bytes(bits // 8, 'big'), 'little')


def htons(x, /):
    return _byte_order(x, 16)


def ntohs(x, /):
    return _byte_order(x, 16)


def htonl(x, /):
    return _byte_order(x, 32)


def ntohl(x, /):
    return _byte_order(x, 32)
