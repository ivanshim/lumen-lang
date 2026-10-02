# Only in-memory streams are carried here: a text one and a binary one.
class UnsupportedOperation(OSError, ValueError):
    pass


def _snapshot(value):
    if type(value) is bytes:
        return value
    if isinstance(value, (bytes, bytearray, memoryview)):
        return bytes(value)
    raise TypeError("a bytes-like object is required, not '%s'" % type(value).__name__)


class StringIO:
    def __init__(self, initial_value='', newline='\n'):
        if newline != "\n":
            raise 'NotImplementedError: alternate newline modes are not supported'
        self.text = initial_value
        self.position = 0
        self.closed = False

    def write(self, text):
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
        if whence == 1:
            if offset != 0:
                raise UnsupportedOperation("can't do nonzero cur-relative seeks")
            return self.position
        if whence == 2:
            if offset != 0:
                raise UnsupportedOperation("can't do nonzero end-relative seeks")
            self.position = len(self.text)
            return self.position
        if whence != 0:
            raise ValueError("unsupported whence (%r)" % (whence,))
        if offset < 0:
            raise ValueError("negative seek position %r" % (offset,))
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

class BytesIO:
    def __init__(self, initial_value=b''):
        self.data = b'' if initial_value is None else _snapshot(initial_value)
        self.position = 0
        self.closed = False

    def write(self, value):
        if self.closed:
            raise ValueError('I/O operation on closed file')
        value = _snapshot(value)
        if not value:
            return 0
        while len(self.data) < self.position:
            self.data += b"\0"
        self.data = self.data[:self.position] + value + self.data[self.position + len(value):]
        self.position += len(value)
        return len(value)

    def getvalue(self):
        self._check()
        return self.data

    def read(self, size=-1):
        self._check()
        if size < 0:
            size = len(self.data) - self.position
        value = self.data[self.position:self.position + size]
        self.position += len(value)
        return value

    def seek(self, offset, whence=0):
        self._check()
        if whence == 1:
            offset += self.position
        elif whence == 2:
            offset += len(self.data)
        elif whence != 0:
            raise ValueError("invalid whence (%d, should be 0, 1 or 2)" % whence)
        if offset < 0:
            if whence == 0:
                raise ValueError("negative seek value %d" % offset)
            offset = 0
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
        result = b''
        while self.position < len(self.data) and (size < 0 or len(result) < size):
            letter = self.read(1)
            result += letter
            if letter == b'\n':
                break
        return result

    def readlines(self, hint=-1):
        self._check()
        lines = []
        length = 0
        while True:
            line = self.readline()
            if line == b'':
                return lines
            lines = [*lines, line]
            length += len(line)
            if hint > 0 and length > hint:
                return lines

    def __iter__(self):
        self._check()
        return self

    def __next__(self):
        line = self.readline()
        if line == b'':
            raise StopIteration
        return line

    def _check(self):
        if self.closed:
            raise ValueError('I/O operation on closed file')
