# Released CJK codec family registry, CPython v3.14.8 Modules/cjkcodecs.
from _multibytecodec import MultibyteCodec
_names = ('iso2022_jp', 'iso2022_jp_1', 'iso2022_jp_2', 'iso2022_jp_2004', 'iso2022_jp_3', 'iso2022_jp_ext', 'iso2022_kr')
_cache = {}
def getcodec(name):
    if not isinstance(name, str):
        raise TypeError('codec name must be str')
    if name not in _names:
        raise LookupError('no such codec is supported')
    if name not in _cache:
        _cache[name] = MultibyteCodec(name)
    return _cache[name]
