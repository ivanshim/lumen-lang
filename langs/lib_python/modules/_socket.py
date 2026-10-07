# Native socket handle exports for the supported socket adapter.
# CPython v3.14.8 Modules/socketmodule.c; PSF License in tests/python/LICENSE.
from socket import SocketType as socket, AF_UNIX, AF_INET, AF_INET6, SOCK_STREAM, SOCK_DGRAM
