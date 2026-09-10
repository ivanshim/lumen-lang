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

def lookup(encoding):
    if encoding in ('utf-8', 'utf8', 'utf_8'):
        return CodecInfo('utf-8')
    if encoding in ('ascii', 'us-ascii', '646'):
        return CodecInfo('ascii')
    if encoding in ('latin-1', 'latin1', 'latin_1', 'iso-8859-1'):
        return CodecInfo('iso8859-1')
    raise 'LookupError: unknown encoding: ' + encoding

def __getattr__(name):
    raise 'NotImplementedError: codecs.' + name + ' needs byte values'
