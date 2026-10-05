# Runtime path bridge to unchanged CPython source-decoding fixtures.
__path__ = [__file__.split('/langs/lib_python/modules/', 1)[0] + '/tests/python-3.14.8/tokenizedata']
