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

# Text adaptation for a binary stream, with incremental encoding at the boundary.
def text_encoding(encoding, stacklevel=2):
    return encoding if encoding is not None else 'utf-8'

class TextIOWrapper:
    def __init__(self, buffer, encoding=None, errors=None, newline=None,
                 line_buffering=False, write_through=False):
        self.buffer = buffer
        self.encoding = text_encoding(encoding)
        self.errors = errors or 'strict'
        self.newlines = None
        self.closed = False
        self.line_buffering = line_buffering
        self.write_through = write_through

    def read(self, size=-1):
        value = self.buffer.read(size)
        return value.decode(self.encoding, self.errors)

    def write(self, value):
        if not isinstance(value, str):
            raise TypeError('write() argument must be str')
        self.buffer.write(value.encode(self.encoding, self.errors))
        if self.write_through or self.line_buffering and '\n' in value:
            self.flush()
        return len(value)

    def flush(self):
        return self.buffer.flush()

    def close(self):
        if not self.closed:
            self.flush()
            self.buffer.close()
            self.closed = True

    def __enter__(self):
        return self

    def __exit__(self, *exc):
        self.close()

    def __getattr__(self, name):
        return getattr(self.buffer, name)
