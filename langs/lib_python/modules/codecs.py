# The byte order marks, which are byte strings and nothing more, so
# they stand here whether or not the encoders below can run.
import sys

BOM_UTF8 = b'\xef\xbb\xbf'
BOM_LE = BOM_UTF16_LE = b'\xff\xfe'
BOM_BE = BOM_UTF16_BE = b'\xfe\xff'
BOM_UTF32_LE = b'\xff\xfe\x00\x00'
BOM_UTF32_BE = b'\x00\x00\xfe\xff'
if sys.byteorder == 'little':
    BOM = BOM_UTF16 = BOM_UTF16_LE
    BOM_UTF32 = BOM_UTF32_LE
else:
    BOM = BOM_UTF16 = BOM_UTF16_BE
    BOM_UTF32 = BOM_UTF32_BE
# The old spellings, kept because programs still write them.
BOM32_LE = BOM_UTF16_LE
BOM32_BE = BOM_UTF16_BE
BOM64_LE = BOM_UTF32_LE
BOM64_BE = BOM_UTF32_BE

# A byte value is not yet part of this definition's run-time floor.
def encode(obj, encoding='utf-8', errors='strict'):
    raise 'NotImplementedError: codecs.encode needs byte values'

def decode(obj, encoding='utf-8', errors='strict'):
    raise 'NotImplementedError: codecs.decode needs byte values'

class CodecInfo:
    def __init__(self, name):
        self.name = name
        self.encode = encode
        self.decode = decode

# The search functions a program has registered. CPython asks each of
# them in turn for a codec it does not already know, and what comes back
# also decides what a text or a byte string does when told to encode or
# decode by that name. Here those two are built into the reader and
# never look at this list, so registering a codec changes what lookup
# answers and nothing else. The list is kept all the same: keeping it is
# the truth about what a program asked for, and refusing to keep it
# would not be.
_searches = []


def register(search_function):
    if not callable(search_function):
        raise 'TypeError: argument must be callable'
    _searches.append(search_function)


def unregister(search_function):
    at = 0
    while at < len(_searches):
        if _searches[at] is search_function:
            _searches.pop(at)
            return
        at += 1


def lookup(encoding):
    if encoding in ('utf-8', 'utf8', 'utf_8'):
        return CodecInfo('utf-8')
    if encoding in ('ascii', 'us-ascii', '646'):
        return CodecInfo('ascii')
    if encoding in ('latin-1', 'latin1', 'latin_1', 'iso-8859-1'):
        return CodecInfo('iso8859-1')
    for search in _searches:
        found = search(encoding)
        if found is not None:
            return found
    raise 'LookupError: unknown encoding: ' + encoding

def __getattr__(name):
    raise 'NotImplementedError: codecs.' + name + ' needs byte values'
