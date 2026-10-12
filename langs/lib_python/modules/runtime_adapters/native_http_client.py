# HTTP client exception base, as defined in CPython v3.14.8 Lib/http/client.py.
class HTTPException(Exception):
    pass

# Default cleartext HTTP port from Lib/http/client.py.
HTTP_PORT = 80
