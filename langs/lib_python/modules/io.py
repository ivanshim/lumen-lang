# Only an in-memory text stream is carried here.
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


# In-memory binary streams use bytes throughout, including EOF and line ends.
class BytesIO:
    def __init__(self, initial_bytes=b''):
        self.data = memoryview(initial_bytes).tobytes()
        self.position = 0
        self.closed = False

    def _check(self):
        if self.closed:
            raise ValueError('I/O operation on closed file.')

    def read(self, size=-1):
        import operator
        self._check()
        size = -1 if size is None else operator.index(size)
        if size < 0:
            size = max(0, len(self.data) - self.position)
        answer = self.data[self.position:self.position + size]
        self.position += len(answer)
        return answer

    def write(self, data):
        self._check()
        data = memoryview(data).tobytes()
        if not data:
            return 0
        if self.position > len(self.data):
            self.data += b'\0' * (self.position - len(self.data))
        self.data = self.data[:self.position] + data + self.data[self.position + len(data):]
        self.position += len(data)
        return len(data)

    def readline(self, size=-1):
        self._check()
        limit = len(self.data) if size is None or size < 0 else self.position + size
        end = self.position
        while end < min(limit, len(self.data)):
            end += 1
            if self.data[end-1] == 10:
                break
        return self.read(max(0, end - self.position))

    def getvalue(self):
        self._check()
        return self.data

    def seek(self, offset, whence=0):
        import operator
        self._check()
        offset = operator.index(offset)
        whence = operator.index(whence)
        if whence == 1:
            offset = max(0, self.position + offset)
        elif whence == 2:
            offset = max(0, len(self.data) + offset)
        elif whence != 0:
            raise ValueError('invalid whence (' + str(whence) + ', should be 0, 1 or 2)')
        if offset < 0:
            raise ValueError('negative seek value ' + str(offset))
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

    def __exit__(self, *args):
        self.close()

    def __iter__(self):
        self._check()
        return self

    def __next__(self):
        result = self.readline()
        if result == b'':
            raise StopIteration
        return result
