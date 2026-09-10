# Packing cannot substitute text or a list for a byte value.
class error(Exception):
    pass

def pack(format, *values):
    raise 'NotImplementedError: struct.pack needs byte values'

def unpack(format, buffer):
    raise 'NotImplementedError: struct.unpack needs byte values'

def calcsize(format):
    raise 'NotImplementedError: struct format layouts are not supported'

class Struct:
    def __init__(self, format):
        raise 'NotImplementedError: Struct needs byte values'

def __getattr__(name):
    raise 'NotImplementedError: struct.' + name + ' needs byte values'
