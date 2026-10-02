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


# Text streams model the native io.TextIOWrapper interface.
class TextIOWrapper(StringIO):
    def __init__(self, buffer, encoding=None, errors=None, newline=None,
                 line_buffering=False, write_through=False):
        self.buffer = buffer
        self.encoding = encoding or 'utf-8'
        self.errors = errors or 'strict'
        self.line_buffering = bool(line_buffering)
        self.write_through = bool(write_through)
        self.name = getattr(buffer, 'name', None)
        self.mode = getattr(buffer, 'mode', 'r')
        self._newline = newline
        data = buffer.read()
        text = data.decode(self.encoding, self.errors)
        if newline is None:
            text = text.replace('\r\n', '\n').replace('\r', '\n')
        StringIO.__init__(self, text)

    def write(self, text):
        self._check()
        if not isinstance(text, str):
            raise TypeError('write() argument must be str')
        if self._newline is not None:
            text = text.replace('\n', self._newline)
        written = self.buffer.write(text.encode(self.encoding, self.errors))
        if self.line_buffering and ('\n' in text or '\r' in text):
            self.flush()
        return len(text)

    def flush(self):
        self._check()
        self.buffer.flush()

    def close(self):
        if not self.closed:
            self.buffer.close()
            self.closed = True

    def readable(self):
        self._check()
        return 'r' in self.mode or '+' in self.mode

    def writable(self):
        self._check()
        return 'w' in self.mode or 'a' in self.mode or '+' in self.mode

    def seekable(self):
        self._check()
        return True
