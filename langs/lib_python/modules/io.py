# In-memory text and binary stream adapters.
class StringIO:
    def __init__(self, initial_value='', newline='\n'):
        if newline != "\n":
            raise 'NotImplementedError: alternate newline modes are not supported'
        self.text = initial_value
        self.position = 0
        self.closed = False

    def write(self, text):
        if not isinstance(text, str):
            raise TypeError('string argument expected, got ' + type(text).__name__)
        if self.closed:
            raise ValueError('I/O operation on closed file')
        while len(self.text) < self.position:
            self.text += "\0"
        self.text = self.text[:self.position] + text + self.text[self.position + len(text):]
        self.position += len(text)
        return len(text)

    def getvalue(self):
        self._check()
        return self.text

    def read(self, size=-1):
        self._check()
        if size < 0:
            size = len(self.text) - self.position
        value = self.text[self.position:self.position + size]
        self.position += len(value)
        return value

    def seek(self, offset, whence=0):
        self._check()
        if whence not in [0, 1, 2]:
            raise 'ValueError: invalid whence'
        if whence != 0 and offset != 0:
            raise 'OSError: cannot do nonzero cur-relative seeks'
        if whence == 2:
            offset += len(self.text)
        elif whence == 1:
            offset += self.position
        if offset < 0:
            raise 'ValueError: negative seek position'
        self.position = offset
        return offset

    def tell(self):
        self._check()
        return self.position

    def flush(self):
        self._check()

    def close(self):
        self.closed = True

    def __enter__(self):
        self._check()
        return self

    def __exit__(self, kind, value, traceback):
        self.close()
        return False

    def readline(self, size=-1):
        if self.closed:
            raise ValueError('I/O operation on closed file')
        result = ''
        while self.position < len(self.text) and (size < 0 or len(result) < size):
            letter = self.read(1)
            result += letter
            if letter == '\n':
                break
        return result


    def readlines(self, hint=-1):
        self._check()
        lines = []
        length = 0
        while True:
            line = self.readline()
            if line == '':
                return lines
            lines = [*lines, line]
            length += len(line)
            if hint > 0 and length > hint:
                return lines

    def __iter__(self):
        self._check()
        return self

    def _line_more(self):
        self._line = self.readline()
        return self._line != ''

    def _line_value(self):
        return self._line

    def __next__(self):
        line = self.readline()
        if line == '':
            raise StopIteration
        return line

    def _check(self):
        if self.closed:
            raise ValueError('I/O operation on closed file')


# Binary memory streams follow CPython v3.14.8 Modules/_io/bytesio.c.
import operator as _operator
_buffer_exports = _export

class UnsupportedOperation(OSError, ValueError):
    pass

class IOBase:
    def fileno(self):
        raise UnsupportedOperation('fileno')

    def isatty(self):
        self._check()
        return False

    def __enter__(self):
        self._check()
        return self

    def __exit__(self, *args):
        self.close()

    def __iter__(self):
        self._check()
        return self

class BufferedIOBase(IOBase):
    def detach(self):
        raise UnsupportedOperation('detach')


def _ssize(value, overflow="Python int too large to convert to C ssize_t"):
    if not isinstance(value, int) and not hasattr(type(value), "__index__"):
        raise TypeError("'" + type(value).__name__ + "' object cannot be interpreted as an integer")
    value = _operator.index(value)
    if value < -9223372036854775808 or value > 9223372036854775807:
        raise OverflowError(overflow)
    return value


def _read_size(value):
    if value is None:
        return -1
    if not isinstance(value, int) and not hasattr(type(value), '__index__'):
        raise TypeError("argument should be integer or None, not '" + type(value).__name__ + "'")
    return _ssize(value, "cannot fit '" + type(value).__name__ + "' into an index-sized integer")


def _contiguous(value):
    if not isinstance(value, (bytes, bytearray, memoryview)):
        try:
            view = memoryview(value)
        except TypeError:
            raise TypeError("a bytes-like object is required, not '" + type(value).__name__ + "'")
    else:
        view = memoryview(value)
    if not view.c_contiguous:
        raise BufferError('memoryview: underlying buffer is not C-contiguous')
    return view


class BytesIO(BufferedIOBase):
    def __new__(cls, *args, **kwargs):
        stream = object.__new__(cls)
        stream._state = [bytearray(), 0, False]
        return stream

    def __init__(self, initial_bytes=b''):
        if not hasattr(self, '_state'):
            self._state = [bytearray(), 0, False]
        self._check_exports()
        self._state[1] = 0
        if initial_bytes is None:
            self._state[0] = bytearray()
        elif isinstance(initial_bytes, bytes):
            self._state = [bytearray(initial_bytes), 0, False]
        else:
            self._state[0] = bytearray()
            BytesIO.write(self, initial_bytes)
            self._state[1] = 0

    @property
    def closed(self):
        return self._state[2]

    def _check(self):
        if self.closed:
            raise ValueError('I/O operation on closed file.')

    def _check_exports(self):
        if _buffer_exports(self._state[0], True):
            raise BufferError('Existing exports of data: object cannot be re-sized')

    def read(self, size=-1, /):
        size = _read_size(size)
        self._check()
        remaining = max(0, len(self._state[0]) - self._state[1])
        if size < 0 or size > remaining:
            size = remaining
        start = self._state[1]
        self._state[1] += size
        return bytes(self._state[0][start:start + size])

    def read1(self, size=-1, /):
        return BytesIO.read(self, size)

    def write(self, data, /):
        view = _contiguous(data)
        self._check()
        self._check_exports()
        data = view.tobytes()
        if not data:
            return 0
        start = self._state[1]
        end = start + len(data)
        if end > 9223372036854775807:
            raise OverflowError('new position too large')
        buffer = self._state[0]
        if start > len(buffer):
            buffer.extend(b'\0' * (start - len(buffer)))
        buffer[start:end] = data
        self._state[1] = end
        return len(data)

    def readline(self, size=-1, /):
        size = _read_size(size)
        self._check()
        start = self._state[1]
        limit = len(self._state[0]) if size < 0 else min(start + size, len(self._state[0]))
        end = start
        while end < limit:
            end += 1
            if self._state[0][end - 1] == 10:
                break
        return BytesIO.read(self, max(0, end - start))

    def readlines(self, size=None, /):
        self._check()
        if size is None:
            size = -1
        elif not isinstance(size, int):
            raise TypeError("integer argument expected, got '" + type(size).__name__ + "'")
        size = _ssize(size)
        result = []
        total = 0
        while True:
            line = BytesIO.readline(self)
            if not line:
                break
            result.append(line)
            total += len(line)
            if size > 0 and total >= size:
                break
        return result

    def writelines(self, lines, /):
        self._check()
        for line in lines:
            BytesIO.write(self, line)
        return None

    def readinto(self, buffer, /):
        try:
            view = memoryview(buffer)
        except TypeError:
            raise TypeError('readinto() argument must be read-write bytes-like object, not ' + ('None' if buffer is None else type(buffer).__name__))
        if view.readonly or not view.c_contiguous:
            raise TypeError('readinto() argument must be read-write bytes-like object, not ' + ('None' if buffer is None else type(buffer).__name__))
        self._check()
        view = view.cast('B')
        value = BytesIO.read(self, view.nbytes)
        view[:len(value)] = value
        return len(value)

    def readinto1(self, buffer, /):
        try:
            view = memoryview(buffer)
        except TypeError:
            raise TypeError('readinto1() argument must be read-write bytes-like object, not ' + ('None' if buffer is None else type(buffer).__name__))
        if view.readonly or not view.c_contiguous:
            raise TypeError('readinto1() argument must be read-write bytes-like object, not ' + type(buffer).__name__)
        view = view.cast('B')
        value = self.read1(view.nbytes)
        if not isinstance(value, bytes):
            raise TypeError('read() should return bytes')
        data = value
        if len(data) > view.nbytes:
            raise ValueError('read() returned too much data: ' + str(view.nbytes) + ' bytes requested, ' + str(len(data)) + ' returned')
        view[:len(data)] = data
        return len(data)

    def getvalue(self):
        self._check()
        return bytes(self._state[0])

    def getbuffer(self):
        self._check()
        return memoryview(self._state[0])

    def seek(self, offset, whence=0, /):
        offset = _ssize(offset)
        whence = _operator.index(whence)
        if whence < -2147483648 or whence > 2147483647:
            raise OverflowError('Python int too large to convert to C int')
        self._check()
        if whence == 0:
            if offset < 0:
                raise ValueError('negative seek value ' + str(offset))
        elif whence in (1, 2):
            offset += self._state[1] if whence == 1 else len(self._state[0])
            if offset > 9223372036854775807:
                raise OverflowError('new position too large')
            offset = max(0, offset)
        else:
            raise ValueError('invalid whence (' + str(whence) + ', should be 0, 1 or 2)')
        self._state[1] = offset
        return offset

    def truncate(self, size=None, /):
        self._check()
        self._check_exports()
        size = self._state[1] if size is None else _ssize(size, "Python int too large to convert to C long")
        if size < 0:
            raise ValueError('negative size value ' + str(size))
        del self._state[0][size:]
        return size

    def tell(self):
        self._check()
        return self._state[1]

    def readable(self):
        self._check()
        return True

    def writable(self):
        self._check()
        return True

    def seekable(self):
        self._check()
        return True

    def flush(self):
        self._check()

    def close(self):
        self._check_exports()
        self._state[0] = bytearray()
        self._state[2] = True

    def __next__(self):
        result = BytesIO.readline(self)
        if not result:
            raise StopIteration
        return result

    def __getstate__(self):
        self._check()
        state = {key: value for key, value in self.__dict__.items() if key != '_state'}
        return (BytesIO.getvalue(self), BytesIO.tell(self), state if state else None)

    def __setstate__(self, state):
        if not isinstance(state, tuple) or len(state) < 3:
            raise TypeError(('_io.BytesIO' if type(self) is BytesIO else type(self).__name__) + '.__setstate__ argument should be 3-tuple, got ' + type(state).__name__)
        self._check_exports()
        self._state[0] = bytearray()
        self._state[1] = 0
        BytesIO.write(self, state[0])
        if not isinstance(state[1], int):
            raise TypeError('second item of state must be an integer, not ' + type(state[1]).__name__)
        position = _ssize(state[1])
        if position < 0:
            raise ValueError('position value cannot be negative')
        self._state[1] = position
        if state[2] is not None:
            if not isinstance(state[2], dict):
                raise TypeError('third item of state should be a dict, got a ' + type(state[2]).__name__)
            self.__dict__.update(state[2])

    def __reduce_ex__(self, protocol, /):
        protocol = _operator.index(protocol)
        if protocol < 2:
            raise TypeError("cannot pickle '" + type(self).__name__ + "' object")
        import copyreg
        return (copyreg.__newobj__, (type(self),), self.__getstate__(), None, None)

    def __reduce__(self):
        raise TypeError("cannot pickle '" + type(self).__name__ + "' object")
