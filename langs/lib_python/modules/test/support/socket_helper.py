# The UNIX socket capability decorator from CPython v3.14.8.
import socket
import unittest
_bind_nix_socket_error = None
def skip_unless_bind_unix_socket(test):
    """Decorator for tests requiring a functional bind() for unix sockets."""
    if not hasattr(socket, 'AF_UNIX'):
        return unittest.skip('No UNIX Sockets')(test)
    global _bind_nix_socket_error
    if _bind_nix_socket_error is None:
        from test.support.os_helper import TESTFN, unlink
        path = TESTFN + "can_bind_unix_socket"
        with socket.socket(socket.AF_UNIX) as sock:
            try:
                sock.bind(path)
                _bind_nix_socket_error = False
            except OSError as e:
                _bind_nix_socket_error = e
            finally:
                unlink(path)
    if _bind_nix_socket_error:
        msg = 'Requires a functional unix bind(): %s' % _bind_nix_socket_error
        return unittest.skip(msg)(test)
    else:
        return test
