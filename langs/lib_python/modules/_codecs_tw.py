# Released CJK codec family registry, CPython v3.14.8 Modules/cjkcodecs.
from _multibytecodec import MultibyteCodec
_names = ('big5', 'cp950')
_cache = {}
def getcodec(name):
    if not isinstance(name, str):
        raise TypeError('codec name must be str')
    if name not in _names:
        raise LookupError('no such codec is supported')
    if name not in _cache:
        _cache[name] = MultibyteCodec(name)
    return _cache[name]
