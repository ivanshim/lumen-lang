# Boolean roots use the standard pickle encodings for protocols 0--5.
# Other values still use this runtime's marshal representation, including
# slices, and can only be read back here. User-defined objects are refused.
import marshal

HIGHEST_PROTOCOL = 5
DEFAULT_PROTOCOL = 5


class PickleError(Exception):
    pass


class PicklingError(PickleError):
    pass


class UnpicklingError(PickleError):
    pass


def _refuse(value):
    raise PicklingError('cannot pickle ' + repr(value) + ': only the built-in kinds are written here')


def _reach(protocol):
    if protocol is None:
        return DEFAULT_PROTOCOL
    if protocol < 0:
        return HIGHEST_PROTOCOL
    if protocol > HIGHEST_PROTOCOL:
        raise ValueError('pickle protocol must be <= ' + str(HIGHEST_PROTOCOL))
    return protocol


def dumps(obj, protocol=None, *, fix_imports=True, buffer_callback=None):
    protocol = _reach(protocol)
    if type(obj) is bool:
        if protocol < 2:
            return b'I01\n.' if obj else b'I00\n.'
        return b'\x80' + bytes([protocol]) + (b'\x88.' if obj else b'\x89.')
    return marshal._write(obj, True, _refuse)


def loads(data, *, fix_imports=True, encoding='ASCII', errors='strict', buffers=None):
    if data == b'I01\n.':
        return True
    if data == b'I00\n.':
        return False
    if len(data) == 4 and data[0] == 128 and 2 <= data[1] <= HIGHEST_PROTOCOL and data[3] == 46:
        if data[2] == 136:
            return True
        if data[2] == 137:
            return False
    return marshal._read(data)


def dump(obj, file, protocol=None, *, fix_imports=True, buffer_callback=None):
    raise 'NotImplementedError: pickle.dump needs a file this runtime cannot write'


def load(file, *, fix_imports=True, encoding='ASCII', errors='strict', buffers=None):
    raise 'NotImplementedError: pickle.load needs a file this runtime cannot read'


class Pickler:
    def __init__(self, file, protocol=None, *, fix_imports=True, buffer_callback=None):
        raise 'NotImplementedError: Pickler needs a file this runtime cannot write'


class Unpickler:
    def __init__(self, file, *, fix_imports=True, encoding='ASCII', errors='strict', buffers=None):
        raise 'NotImplementedError: Unpickler needs a file this runtime cannot read'


def __getattr__(name):
    raise 'NotImplementedError: pickle.' + name + ' is not supported'
