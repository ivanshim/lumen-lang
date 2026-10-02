# Only an in-memory text stream is carried here.
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


# An in-memory binary stream, faithful to CPython's pure-Python
# _pyio.BytesIO: the initial value is copied in, the buffer grows with
# null bytes on a write past its end, a seek may not move before the
# beginning, and getvalue hands back immutable bytes.
class BytesIO:
    def __init__(self, initial_bytes=None):
        self.data = b''
        if initial_bytes is not None:
            with memoryview(initial_bytes) as view:
                self.data = bytes(view)
        self.position = 0
        self.closed = False

    def write(self, value):
        if isinstance(value, str):
            raise TypeError("can't write str to binary stream")
        with memoryview(value) as view:
            if self.closed:
                raise ValueError('write to closed file')
            size = view.nbytes
            if size == 0:
                return 0
            position = self.position
            data = self.data
            if position > len(data):
                data = data + bytes(position - len(data))
            self.data = data[:position] + bytes(view) + data[position + size:]
            self.position = position + size
        return size

    def getvalue(self):
        if self.closed:
            raise ValueError('getvalue on closed file')
        return self.data

    def read(self, size=-1):
        if self.closed:
            raise ValueError('read from closed file')
        if size is None:
            size = -1
        else:
            try:
                size = size.__index__()
            except AttributeError:
                raise TypeError('%r is not an integer' % (size,))
        if size < 0:
            size = len(self.data)
        if len(self.data) <= self.position:
            return b''
        newpos = min(len(self.data), self.position + size)
        value = self.data[self.position:newpos]
        self.position = newpos
        return value

    def seek(self, offset, whence=0):
        if self.closed:
            raise ValueError('seek on closed file')
        try:
            offset = offset.__index__()
        except AttributeError:
            raise TypeError('%r is not an integer' % (offset,))
        if whence == 0:
            if offset < 0:
                raise ValueError('negative seek position %r' % (offset,))
            self.position = offset
        elif whence == 1:
            self.position = max(0, self.position + offset)
        elif whence == 2:
            self.position = max(0, len(self.data) + offset)
        else:
            raise ValueError('unsupported whence value')
        return self.position

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
        if size is None:
            size = -1
        else:
            try:
                size = size.__index__()
            except AttributeError:
                raise TypeError('%r is not an integer' % (size,))
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

    def _check(self):
        if self.closed:
            raise ValueError('I/O operation on closed file')
