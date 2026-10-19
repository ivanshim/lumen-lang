# Unicode databases generated from official release data; see scripts/generate_unicodedata.py.
unidata_version = '16.0.0'
_token = object()

class UCD:
    # Bind native methods once, keeping whole-range calls outside Python wrappers.
    def __init__(self, version=None, token=None):
        if token is not _token:
            raise TypeError("cannot create 'unicodedata.UCD' instances")
        self.unidata_version = version
        old = int(version == '3.2.0')
        methods = ('category', 'bidirectional', 'combining', 'mirrored',
                   'east_asian_width', 'decimal', 'digit', 'numeric',
                   'decomposition', 'name', 'lookup', 'normalize', 'is_normalized')
        for index, method in enumerate(methods):
            setattr(self, method, __sre_native(18, index, old))

_current = UCD(unidata_version, _token)
ucd_3_2_0 = UCD('3.2.0', _token)
category = _current.category
bidirectional = _current.bidirectional
combining = _current.combining
mirrored = _current.mirrored
east_asian_width = _current.east_asian_width
decimal = _current.decimal
digit = _current.digit
numeric = _current.numeric
# Preserve the native callable's metadata and its existing entry point.
decomposition = __unicode_decomposition
name = _current.name
lookup = _current.lookup
normalize = _current.normalize
is_normalized = _current.is_normalized
