# Minimal http.server adapter; source CPython v3.14.8 Lib/http/server.py.
# This build has no working socket server, so the HTTP test classes in the
# reference suite skip through test.support.has_socket_support before any of
# these classes is used. The names exist so a program that imports the module
# imports cleanly; constructing a server reports the missing facility.
class BaseHTTPRequestHandler:

    def log_message(self, format, *args):
        pass


class SimpleHTTPRequestHandler(BaseHTTPRequestHandler):
    pass


class CGIHTTPRequestHandler(SimpleHTTPRequestHandler):
    pass


class HTTPServer:

    def __init__(self, server_address, RequestHandlerClass,
                 bind_and_activate=True):
        raise NotImplementedError(
            "http.server.HTTPServer is unavailable: no socket server")


class ThreadingHTTPServer(HTTPServer):
    pass
