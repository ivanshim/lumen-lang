# Values are kept in a versioned in-memory text format.
HIGHEST_PROTOCOL = 5
DEFAULT_PROTOCOL = 5

class PickleError(Exception):
    pass

class PicklingError(PickleError):
    pass

class UnpicklingError(PickleError):
    pass


def dumps(obj, protocol=None, *, fix_imports=True, buffer_callback=None):
    if protocol is None:
        protocol = DEFAULT_PROTOCOL
    if not isinstance(protocol, type(1)):
        raise 'TypeError: an integer is required'
    if protocol < 0:
        protocol = HIGHEST_PROTOCOL
    if protocol > HIGHEST_PROTOCOL:
        raise 'ValueError: pickle protocol must be <= 5'
    if buffer_callback is not None:
        raise 'NotImplementedError: pickle buffers are not supported'
    return __pickle_pack(obj)


def loads(data, *, fix_imports=True, encoding='ASCII', errors='strict', buffers=None):
    if buffers is not None or encoding != 'ASCII' or errors != 'strict':
        raise 'NotImplementedError: pickle buffer and encoding options are not supported'
    return __pickle_unpack(data)


def dump(obj, file, protocol=None, *, fix_imports=True, buffer_callback=None):
    file.write(dumps(obj, protocol, fix_imports=fix_imports, buffer_callback=buffer_callback) + "\n")


def load(file, *, fix_imports=True, encoding='ASCII', errors='strict', buffers=None):
    data = ''
    while True:
        letter = file.read(1)
        if letter == '' or letter == '\n':
            break
        data += letter
    if data == '':
        raise 'EOFError: Ran out of input'
    return loads(data, fix_imports=fix_imports, encoding=encoding, errors=errors, buffers=buffers)


class Pickler:
    def __init__(self, file, protocol=None, *, fix_imports=True, buffer_callback=None):
        self.file = file
        self.protocol = protocol
        self.fix_imports = fix_imports
        self.buffer_callback = buffer_callback

    def dump(self, obj):
        dump(obj, self.file, self.protocol, fix_imports=self.fix_imports, buffer_callback=self.buffer_callback)

    def clear_memo(self):
        pass


class Unpickler:
    def __init__(self, file, *, fix_imports=True, encoding='ASCII', errors='strict', buffers=None):
        self.file = file
        self.fix_imports = fix_imports
        self.encoding = encoding
        self.errors = errors
        self.buffers = buffers

    def load(self):
        return load(self.file, fix_imports=self.fix_imports, encoding=self.encoding, errors=self.errors, buffers=self.buffers)
