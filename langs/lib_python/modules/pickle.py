# A value handed out as bytes and taken back as itself. What is written
# here is what marshal writes -- this runtime's own writing, not the
# reference implementation's pickle protocol -- with a slice taken as
# well, so a stream written here is read back only here and says so
# rather than pretending to a protocol it does not have. There is one
# form, so a protocol number is taken and looked at for range and has no
# other effect. A thing of a class of its own is refused instead of
# being written by the class's name, which nothing here can find again.
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
    _reach(protocol)
    return marshal._write(obj, True, _refuse)


def loads(data, *, fix_imports=True, encoding='ASCII', errors='strict', buffers=None):
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
